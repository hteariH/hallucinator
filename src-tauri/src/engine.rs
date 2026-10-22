use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};

pub fn server_args(model: &Path, port: u16, gpu: bool) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-m".into(), model.to_string_lossy().into_owned(),
        "--host".into(), "127.0.0.1".into(),
        "--port".into(), port.to_string(),
        "-c".into(), "2048".into(),
        "-np".into(), "1".into(),
        "--no-webui".into(),
        "-ngl".into(), if gpu { "99" } else { "0" }.into(),
    ];
    if !gpu {
        // Do not even initialise the GPU backend: a broken driver is often why GPU start failed.
        args.extend(["--device".into(), "none".into()]);
    }
    args
}

pub fn free_port() -> std::io::Result<u16> {
    Ok(std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

/// PID of the llama-server being started/run, readable without taking the engine lock
/// (so the exit handler can kill a server that is still starting).
pub type PidSlot = Arc<Mutex<Option<u32>>>;

/// Force-kills a process tree by PID. Never blocks on anything but the kill itself.
pub fn kill_pid(pid: u32) {
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let _ = cmd.output();
}

/// Windows Job Object with KILL_ON_JOB_CLOSE: when the app process dies for any reason
/// (panic, Task Manager, crash) the OS closes the handle and kills every assigned process.
#[cfg(windows)]
pub struct Job(isize);

#[cfg(windows)]
impl Job {
    pub fn new() -> Result<Job, String> {
        use windows::Win32::System::JobObjects::*;
        unsafe {
            let h = CreateJobObjectW(None, windows::core::PCWSTR::null()).map_err(|e| e.to_string())?;
            let job = Job(h.0 as isize); // owned from here on: Drop closes it
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .map_err(|e| e.to_string())?;
            Ok(job)
        }
    }

    /// `process` is a raw process HANDLE.
    pub fn assign(&self, process: *mut std::ffi::c_void) -> Result<(), String> {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::JobObjects::AssignProcessToJobObject;
        unsafe { AssignProcessToJobObject(HANDLE(self.0 as *mut _), HANDLE(process)).map_err(|e| e.to_string()) }
    }
}

#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        unsafe {
            let _ = CloseHandle(HANDLE(self.0 as *mut _));
        }
    }
}

/// One job per app process, alive until the process ends.
#[cfg(windows)]
fn app_job() -> &'static Result<Job, String> {
    static JOB: std::sync::OnceLock<Result<Job, String>> = std::sync::OnceLock::new();
    JOB.get_or_init(Job::new)
}

/// Puts the freshly spawned server into the app job. Failure is logged, never fatal.
fn bind_to_app_job(child: &Child, log: &Path) {
    #[cfg(windows)]
    {
        let res = match (app_job(), child.raw_handle()) {
            (Ok(job), Some(h)) => job.assign(h),
            (Err(e), _) => Err(format!("job object unavailable: {e}")),
            (_, None) => Err("child handle unavailable".to_string()),
        };
        if let Err(e) = res {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log) {
                let _ = writeln!(f, "[llm-calc] could not assign llama-server to the job object: {e}");
            }
        }
    }
    #[cfg(not(windows))]
    let _ = (child, log);
}

pub struct Engine {
    pub port: u16,
    pub gpu: bool,
    child: Option<Child>,
    pid: PidSlot,
}

const READY_TIMEOUT: Duration = Duration::from_secs(180);

impl Engine {
    /// Starts llama-server on the GPU; if that fails, once more on the CPU.
    pub async fn start(exe: &Path, model: &Path, log: &Path, pid: &PidSlot) -> Result<Engine, String> {
        match Self::start_once(exe, model, log, true, pid).await {
            Ok(e) => Ok(e),
            Err(gpu_err) => Self::start_once(exe, model, log, false, pid)
                .await
                .map_err(|cpu_err| format!("GPU: {gpu_err}; CPU: {cpu_err}")),
        }
    }

    async fn start_once(exe: &Path, model: &Path, log: &Path, gpu: bool, pid: &PidSlot) -> Result<Engine, String> {
        if !exe.is_file() {
            return Err(format!("engine not found at {}", exe.display()));
        }
        if let Some(dir) = log.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let log_file = std::fs::OpenOptions::new().create(true).append(true).open(log).map_err(|e| e.to_string())?;
        let port = free_port().map_err(|e| e.to_string())?;
        let mut cmd = Command::new(exe);
        cmd.args(server_args(model, port, gpu))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log_file)
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let child = cmd.spawn().map_err(|e| e.to_string())?;
        bind_to_app_job(&child, log);
        *pid.lock().unwrap() = child.id();
        let mut engine = Engine { port, gpu, child: Some(child), pid: pid.clone() };
        match engine.wait_ready().await {
            Ok(()) => Ok(engine),
            Err(e) => {
                engine.stop().await;
                Err(e)
            }
        }
    }

    async fn wait_ready(&mut self) -> Result<(), String> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(2)).build().map_err(|e| e.to_string())?;
        let url = format!("http://127.0.0.1:{}/health", self.port);
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(child) = self.child.as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(format!("engine exited ({status}), see engine.log"));
                }
            }
            if let Ok(r) = client.get(&url).send().await {
                if r.status().is_success() {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        Err(format!("engine did not become ready in {} s", READY_TIMEOUT.as_secs()))
    }

    pub fn is_alive(&mut self) -> bool {
        self.child.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None)))
    }

    pub async fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill().await;
        }
        *self.pid.lock().unwrap() = None;
    }

    pub fn kill_now(&mut self) {
        if let Some(c) = self.child.as_mut() {
            let _ = c.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_args_offload_everything() {
        let a = server_args(Path::new("m.gguf"), 1234, true);
        assert!(a.windows(2).any(|w| w == ["-ngl", "99"]));
        assert!(a.windows(2).any(|w| w == ["--port", "1234"]));
        assert!(a.windows(2).any(|w| w == ["--host", "127.0.0.1"]));
        assert!(!a.contains(&"--device".to_string()));
    }

    #[test]
    fn cpu_args_disable_gpu() {
        let a = server_args(Path::new("m.gguf"), 1, false);
        assert!(a.windows(2).any(|w| w == ["-ngl", "0"]));
        assert!(a.windows(2).any(|w| w == ["--device", "none"]));
    }

    #[test]
    fn free_port_is_nonzero() {
        assert_ne!(free_port().unwrap(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn app_job_is_created() {
        assert!(app_job().is_ok(), "{:?}", app_job().as_ref().err());
    }

    #[cfg(windows)]
    #[test]
    fn closing_job_kills_assigned_process() {
        use std::os::windows::io::AsRawHandle;
        use std::os::windows::process::CommandExt;
        let job = Job::new().unwrap();
        let mut child = std::process::Command::new("cmd")
            .args(["/c", "ping", "-n", "30", "127.0.0.1"])
            .stdout(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .unwrap();
        job.assign(child.as_raw_handle()).unwrap();
        assert!(child.try_wait().unwrap().is_none());
        drop(job);
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut exited = false;
        while Instant::now() < deadline {
            if child.try_wait().unwrap().is_some() {
                exited = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if !exited {
            let _ = child.kill();
        }
        assert!(exited, "process survived closing its job");
    }

    #[tokio::test]
    async fn missing_exe_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let r = Engine::start(&dir.path().join("nope.exe"), Path::new("m.gguf"), &dir.path().join("log"), &PidSlot::default()).await;
        assert!(r.err().unwrap().contains("engine not found"));
    }

    /// Real engine. Needs LLM_CALC_SERVER (path to llama-server.exe) and LLM_CALC_MODEL (path to a GGUF).
    #[tokio::test]
    #[ignore]
    async fn real_engine_answers() {
        let exe = std::env::var("LLM_CALC_SERVER").unwrap();
        let model = std::env::var("LLM_CALC_MODEL").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut e = Engine::start(Path::new(&exe), Path::new(&model), &dir.path().join("engine.log"), &PidSlot::default()).await.unwrap();
        assert!(e.is_alive());
        let r = crate::ask::stream(&reqwest::Client::new(), e.port, "2+2", &Default::default(), |_| {},
                                   tokio_util::sync::CancellationToken::new()).await.unwrap();
        println!("model said: {:?} (gpu: {})", r.text, e.gpu);
        assert!(!r.text.is_empty());
        e.stop().await;
        assert!(!e.is_alive());
    }
}
