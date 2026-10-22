use crate::download;
use crate::engine::Engine;
use crate::models;
use crate::settings::{self, Sampling, Settings};
use crate::vram;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
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
        Ok(Self {
            models_dir: data.join("models"),
            settings: Mutex::new(settings::load(&settings_path)),
            settings_path,
            server_exe: app.path().resource_dir()?.join("llama").join("llama-server.exe"),
            log_path: data.join("engine.log"),
            engine: Default::default(),
            status: Mutex::new(EngineStatus::NoModel),
            current_ask: Default::default(),
            downloading: Default::default(),
            client: reqwest::Client::new(),
            vram: vram::max_dedicated_vram(),
        })
    }

    /// Called from the exit handler, outside the async runtime.
    pub fn kill_engine_now(&self) {
        let mut guard = match self.engine.try_lock() {
            Ok(g) => g,
            Err(_) => self.engine.blocking_lock(),
        };
        if let Some(e) = guard.as_mut() {
            e.kill_now();
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

/// (Re)starts the engine with the active model, reporting progress as engine-status events.
pub async fn restart_engine_inner(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut guard = state.engine.lock().await;
    let restarting = guard.is_some();
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
    set_status(app, if restarting { EngineStatus::Restarting } else { EngineStatus::Starting });
    match Engine::start(&state.server_exe, &path, &state.log_path).await {
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
    restart_engine_inner(app).await;
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
        restart_engine_inner(&app).await;
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
    models::delete(&state.models_dir, spec).map_err(|e| e.to_string())?;
    if was_active {
        let pick = state.first_downloaded();
        state.settings.lock().unwrap().active_model = pick;
        state.save_settings();
        restart_engine_inner(&app).await;
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
    state.settings.lock().unwrap().active_model = Some(id);
    state.save_settings();
    restart_engine_inner(&app).await;
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
    restart_engine_inner(&app).await;
}

#[tauri::command]
pub async fn ask(app: AppHandle, expression: String) -> Result<crate::ask::AskResult, String> {
    let state = app.state::<AppState>();
    let port = match state.engine.lock().await.as_ref() {
        Some(e) => e.port,
        None => return Err("The brain is not ready yet".into()),
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

    result.or_else(|e| {
        let alive = state.engine.try_lock().map(|mut g| g.as_mut().is_some_and(|e| e.is_alive())).unwrap_or(true);
        if alive {
            return Err(e);
        }
        let h = app.clone();
        tauri::async_runtime::spawn(async move { restart_engine_inner(&h).await });
        Err("brain crashed, restarting…".into())
    })
}

#[tauri::command]
pub fn cancel_ask(state: State<AppState>) {
    if let Some(t) = state.current_ask.lock().unwrap().as_ref() {
        t.cancel();
    }
}
