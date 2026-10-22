use std::path::Path;
use std::process::Stdio;
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

pub struct Engine {
    pub port: u16,
    pub gpu: bool,
    child: Option<Child>,
}

const READY_TIMEOUT: Duration = Duration::from_secs(180);

impl Engine {
    /// Starts llama-server on the GPU; if that fails, once more on the CPU.
    pub async fn start(exe: &Path, model: &Path, log: &Path) -> Result<Engine, String> {
        match Self::start_once(exe, model, log, true).await {
            Ok(e) => Ok(e),
            Err(gpu_err) => Self::start_once(exe, model, log, false)
                .await
                .map_err(|cpu_err| format!("GPU: {gpu_err}; CPU: {cpu_err}")),
        }
    }

    async fn start_once(exe: &Path, model: &Path, log: &Path, gpu: bool) -> Result<Engine, String> {
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
        let mut engine = Engine { port, gpu, child: Some(child) };
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

    #[tokio::test]
    async fn missing_exe_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let r = Engine::start(&dir.path().join("nope.exe"), Path::new("m.gguf"), &dir.path().join("log")).await;
        assert!(r.err().unwrap().contains("engine not found"));
    }

    /// Real engine. Needs LLM_CALC_SERVER (path to llama-server.exe) and LLM_CALC_MODEL (path to a GGUF).
    #[tokio::test]
    #[ignore]
    async fn real_engine_answers() {
        let exe = std::env::var("LLM_CALC_SERVER").unwrap();
        let model = std::env::var("LLM_CALC_MODEL").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut e = Engine::start(Path::new(&exe), Path::new(&model), &dir.path().join("engine.log")).await.unwrap();
        assert!(e.is_alive());
        let r = crate::ask::stream(&reqwest::Client::new(), e.port, "2+2", &Default::default(), |_| {},
                                   tokio_util::sync::CancellationToken::new()).await.unwrap();
        println!("model said: {:?} (gpu: {})", r.text, e.gpu);
        assert!(!r.text.is_empty());
        e.stop().await;
        assert!(!e.is_alive());
    }
}
