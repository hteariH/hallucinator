use crate::download;
use crate::engine::{self, Engine, PidSlot};
use crate::models;
use crate::settings::{self, Sampling, Settings};
use crate::vram;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum EngineStatus {
    NoModel,
    Starting,
    Ready,
    Failed { reason: String },
    Restarting,
}

pub struct AppState {
    models_dir: PathBuf,
    settings_path: PathBuf,
    server_exe: PathBuf,
    log_path: PathBuf,
    settings: Mutex<Settings>,
    engine: tokio::sync::Mutex<Option<Engine>>,
    engine_pid: PidSlot,
    status: Mutex<EngineStatus>,
    current_ask: Mutex<Option<CancellationToken>>,
    downloading: Mutex<HashSet<String>>,
    client: reqwest::Client,
    vram: u64,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let data = app.path().app_local_data_dir()?;
        let settings_path = data.join("settings.json");
        let models_dir = data.join("models");
        let settings = settings::load(&settings_path);
        let has_model = settings
            .active_model
            .as_deref()
            .and_then(models::find)
            .is_some_and(|m| models::model_path(&models_dir, m).is_file());
        Ok(Self {
            models_dir,
            settings: Mutex::new(settings),
            settings_path,
            server_exe: app.path().resource_dir()?.join("llama").join("llama-server.exe"),
            log_path: data.join("engine.log"),
            engine: Default::default(),
            engine_pid: Default::default(),
            status: Mutex::new(if has_model { EngineStatus::Starting } else { EngineStatus::NoModel }),
            current_ask: Default::default(),
            downloading: Default::default(),
            client: reqwest::Client::new(),
            vram: vram::max_dedicated_vram(),
        })
    }

    /// Called from the exit handler, outside the async runtime.
    pub fn kill_engine_now(&self) {
        // Never wait for the engine lock: it can be held for minutes while the server starts.
        match self.engine.try_lock() {
            Ok(mut guard) => {
                if let Some(e) = guard.as_mut() {
                    e.kill_now();
                }
            }
            Err(_) => {
                if let Some(pid) = self.engine_pid.lock().unwrap().take() {
                    engine::kill_pid(pid);
                }
            }
        }
    }

    fn save_settings(&self) {
        let s = self.settings.lock().unwrap().clone();
        let _ = settings::save(&self.settings_path, &s);
    }

    fn first_downloaded(&self) -> Option<String> {
        models::list(&self.models_dir).into_iter().find(|m| m.downloaded).map(|m| m.id)
    }
}

fn set_status(app: &AppHandle, st: EngineStatus) {
    *app.state::<AppState>().status.lock().unwrap() = st.clone();
    let _ = app.emit("engine-status", st);
}

/// Why the engine is being (re)started; only a crash is reported to the user as "restarting".
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RestartReason {
    Boot,
    Switch,
    Crash,
    Retry,
}

fn starting_status(reason: RestartReason) -> EngineStatus {
    match reason {
        RestartReason::Crash => EngineStatus::Restarting,
        _ => EngineStatus::Starting,
    }
}

/// (Re)starts the engine with the active model, reporting progress as engine-status events.
pub async fn restart_engine_inner(app: &AppHandle, reason: RestartReason) {
    let state = app.state::<AppState>();
    let mut guard = state.engine.lock().await;
    if let Some(mut old) = guard.take() {
        old.stop().await;
    }
    let active = state.settings.lock().unwrap().active_model.clone();
    let Some(spec) = active.as_deref().and_then(models::find) else {
        return set_status(app, EngineStatus::NoModel);
    };
    let path = models::model_path(&state.models_dir, spec);
    if !path.is_file() {
        return set_status(app, EngineStatus::NoModel);
    }
    set_status(app, starting_status(reason));
    match Engine::start(&state.server_exe, &path, &state.log_path, &state.engine_pid).await {
        Ok(e) => {
            *guard = Some(e);
            set_status(app, EngineStatus::Ready);
        }
        Err(reason) => set_status(app, EngineStatus::Failed { reason }),
    }
}

/// On launch: make sure the active model exists (else pick any downloaded one), then start.
pub async fn boot(app: &AppHandle) {
    let state = app.state::<AppState>();
    let valid = {
        let s = state.settings.lock().unwrap();
        s.active_model
            .as_deref()
            .and_then(models::find)
            .is_some_and(|m| models::model_path(&state.models_dir, m).is_file())
    };
    if !valid {
        let pick = state.first_downloaded();
        state.settings.lock().unwrap().active_model = pick;
        state.save_settings();
    }
    restart_engine_inner(app, RestartReason::Boot).await;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    models: Vec<models::ModelInfo>,
    active_model: Option<String>,
    sampling: Sampling,
    engine: EngineStatus,
    recommended_model: &'static str,
    downloading: Vec<String>,
}

#[tauri::command]
pub fn get_snapshot(state: State<AppState>) -> Snapshot {
    let s = state.settings.lock().unwrap().clone();
    Snapshot {
        models: models::list(&state.models_dir),
        active_model: s.active_model,
        sampling: s.sampling,
        engine: state.status.lock().unwrap().clone(),
        recommended_model: vram::recommended_model(state.vram),
        downloading: state.downloading.lock().unwrap().iter().cloned().collect(),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    id: String,
    done: u64,
    total: u64,
}

#[tauri::command]
pub async fn download_model(app: AppHandle, id: String) -> Result<(), String> {
    let spec = models::find(&id).ok_or("unknown model")?;
    let state = app.state::<AppState>();
    if !state.downloading.lock().unwrap().insert(id.clone()) {
        return Err("already downloading".into());
    }
    let dest = models::model_path(&state.models_dir, spec);
    let mut last_pct = u64::MAX;
    let result = download::download(&state.client, spec.url, &dest, spec.sha256, |done| {
        let pct = done * 100 / spec.size;
        if pct != last_pct {
            last_pct = pct;
            let _ = app.emit("download-progress", Progress { id: id.clone(), done, total: spec.size });
        }
    })
    .await;
    state.downloading.lock().unwrap().remove(&id);
    result.map_err(|e| e.to_string())?;

    let first = {
        let mut s = state.settings.lock().unwrap();
        let first = s.active_model.is_none();
        if first {
            s.active_model = Some(id.clone());
        }
        first
    };
    if first {
        state.save_settings();
        // Do not hold the call open: the calculator's "Waking up the brain…" banner follows the status events.
        set_status(&app, starting_status(RestartReason::Switch)); // visible before this call returns
        let h = app.clone();
        tauri::async_runtime::spawn(async move { restart_engine_inner(&h, RestartReason::Switch).await });
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_model(app: AppHandle, id: String) -> Result<(), String> {
    let spec = models::find(&id).ok_or("unknown model")?;
    let state = app.state::<AppState>();
    if state.downloading.lock().unwrap().contains(&id) {
        return Err("this model is still downloading".into());
    }
    let was_active = state.settings.lock().unwrap().active_model.as_deref() == Some(id.as_str());
    if was_active {
        if let Some(mut e) = state.engine.lock().await.take() {
            e.stop().await;
        }
    }
    if let Err(e) = models::delete(&state.models_dir, spec) {
        if was_active {
            restart_engine_inner(&app, RestartReason::Switch).await;
        }
        return Err(e.to_string());
    }
    if was_active {
        let pick = state.first_downloaded();
        state.settings.lock().unwrap().active_model = pick;
        state.save_settings();
        restart_engine_inner(&app, RestartReason::Switch).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn select_model(app: AppHandle, id: String) -> Result<(), String> {
    let spec = models::find(&id).ok_or("unknown model")?;
    let state = app.state::<AppState>();
    if !models::model_path(&state.models_dir, spec).is_file() {
        return Err("download this model first".into());
    }
    let unchanged = state.settings.lock().unwrap().active_model.as_deref() == Some(id.as_str())
        && matches!(*state.status.lock().unwrap(), EngineStatus::Ready);
    if unchanged {
        return Ok(());
    }
    state.settings.lock().unwrap().active_model = Some(id);
    state.save_settings();
    // Return at once; engine-status events drive the UI.
    set_status(&app, starting_status(RestartReason::Switch)); // visible before this call returns
    let h = app.clone();
    tauri::async_runtime::spawn(async move { restart_engine_inner(&h, RestartReason::Switch).await });
    Ok(())
}

#[tauri::command]
pub fn save_sampling(state: State<AppState>, sampling: Sampling) -> Sampling {
    let clamped = sampling.clamped();
    state.settings.lock().unwrap().sampling = clamped.clone();
    state.save_settings();
    clamped
}

#[tauri::command]
pub async fn restart_engine(app: AppHandle) {
    restart_engine_inner(&app, RestartReason::Retry).await;
}

#[tauri::command]
pub async fn ask(app: AppHandle, expression: String) -> Result<crate::ask::AskResult, String> {
    const NOT_READY: &str = "The brain is not ready yet";
    let state = app.state::<AppState>();
    // Never wait on the engine lock: during (re)start it is held for a long time.
    if !matches!(*state.status.lock().unwrap(), EngineStatus::Ready) {
        return Err(NOT_READY.into());
    }
    let port = match state.engine.try_lock() {
        Ok(g) => match g.as_ref() {
            Some(e) => e.port,
            None => return Err(NOT_READY.into()),
        },
        Err(_) => return Err(NOT_READY.into()),
    };
    let token = CancellationToken::new();
    {
        let mut current = state.current_ask.lock().unwrap();
        if current.is_some() {
            return Err("Already thinking".into());
        }
        *current = Some(token.clone());
    }
    let sampling = state.settings.lock().unwrap().sampling.clone();
    let result = crate::ask::stream(&state.client, port, &expression, &sampling, |t| {
        let _ = app.emit("ask-token", t.to_string());
    }, token)
    .await;
    *state.current_ask.lock().unwrap() = None;

    let Err(e) = result else { return result };
    // The process may need a moment to be reported dead; poll briefly, never wait unboundedly.
    for _ in 0..10 {
        let Ok(mut guard) = tokio::time::timeout(Duration::from_millis(100), state.engine.lock()).await else {
            return Err(e); // a restart is in progress
        };
        match guard.as_mut().map(|eng| eng.is_alive()) {
            None => return Err(e),
            Some(false) => {
                drop(guard);
                let h = app.clone();
                tauri::async_runtime::spawn(async move { restart_engine_inner(&h, RestartReason::Crash).await });
                return Err("brain crashed, restarting…".into());
            }
            Some(true) => {}
        }
        drop(guard);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(e)
}

#[tauri::command]
pub fn cancel_ask(state: State<AppState>) {
    if let Some(t) = state.current_ask.lock().unwrap().as_ref() {
        t.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_crash_reports_restarting() {
        assert!(matches!(starting_status(RestartReason::Crash), EngineStatus::Restarting));
        for r in [RestartReason::Boot, RestartReason::Switch, RestartReason::Retry] {
            assert!(matches!(starting_status(r), EngineStatus::Starting), "{r:?}");
        }
    }
}
