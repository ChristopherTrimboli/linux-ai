//! Desktop backend: wires the shared `la-core` engine to Tauri commands and a
//! streaming IPC channel. Tool approvals round-trip to the UI via a pending map.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use la_core::tools::{ApprovalDecision, ApprovalRequest, Approver};
use la_core::{Agent, AgentEvent, Config, Conversation, Message, Store};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::{mpsc, oneshot};

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>;

pub struct AppState {
    store: Store,
    config: Mutex<Config>,
    pending: PendingMap,
}

#[derive(Serialize)]
pub struct ProviderStatus {
    name: String,
    kind: String,
    base_url: String,
    has_key: bool,
    models: Vec<String>,
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
fn list_conversations(state: State<'_, AppState>) -> Result<Vec<Conversation>, String> {
    state.store.list_conversations().map_err(err)
}

#[tauri::command]
fn create_conversation(state: State<'_, AppState>) -> Result<Conversation, String> {
    state.store.create_conversation("New chat").map_err(err)
}

#[tauri::command]
fn load_messages(state: State<'_, AppState>, id: String) -> Result<Vec<Message>, String> {
    state.store.load_messages(&id).map_err(err)
}

#[tauri::command]
fn rename_conversation(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> Result<(), String> {
    state.store.rename_conversation(&id, &title).map_err(err)
}

#[tauri::command]
fn delete_conversation(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.store.delete_conversation(&id).map_err(err)
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn set_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    config.save().map_err(err)?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
fn provider_status(state: State<'_, AppState>) -> Vec<ProviderStatus> {
    let cfg = state.config.lock().unwrap();
    cfg.providers
        .iter()
        .map(|(name, pc)| ProviderStatus {
            name: name.clone(),
            kind: format!("{:?}", pc.kind).to_lowercase(),
            base_url: pc.effective_base_url(),
            has_key: la_core::secrets::resolve_api_key(name, pc).is_some(),
            models: pc.models.clone(),
        })
        .collect()
}

#[tauri::command]
fn set_api_key(provider: String, key: String) -> Result<(), String> {
    la_core::secrets::store_api_key(&provider, &key).map_err(err)
}

#[tauri::command]
fn respond_approval(state: State<'_, AppState>, id: String, approved: bool) {
    let sender = state.pending.lock().unwrap().remove(&id);
    if let Some(sender) = sender {
        let _ = sender.send(approved);
    }
}

fn desktop_approver(pending: PendingMap, tx: mpsc::UnboundedSender<AgentEvent>) -> Approver {
    Arc::new(move |req: ApprovalRequest| {
        let pending = pending.clone();
        let tx = tx.clone();
        Box::pin(async move {
            let id = uuid::Uuid::new_v4().to_string();
            let (otx, orx) = oneshot::channel::<bool>();
            {
                pending.lock().unwrap().insert(id.clone(), otx);
            }
            let _ = tx.send(AgentEvent::ApprovalRequired {
                id: id.clone(),
                tool: req.tool,
                summary: req.summary,
            });
            match orx.await {
                Ok(true) => ApprovalDecision::Approve,
                _ => ApprovalDecision::Deny,
            }
        })
    })
}

#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    conversation_id: String,
    text: String,
    provider: Option<String>,
    model: Option<String>,
    on_event: Channel<AgentEvent>,
) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let store = state.store.clone();
    let pending = state.pending.clone();

    let agent = Agent::from_config(&config, store, provider.as_deref(), model.as_deref())
        .map_err(err)?;
    let agent = Arc::new(agent);

    let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();
    let approver = desktop_approver(pending, tx.clone());

    let agent2 = agent.clone();
    tokio::spawn(async move {
        let _ = agent2.run_turn(&conversation_id, &text, approver, tx).await;
    });

    while let Some(event) = rx.recv().await {
        if on_event.send(event).is_err() {
            break;
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::load().expect("failed to load config");
    let store = Store::open_default().expect("failed to open data store");

    tauri::Builder::default()
        .manage(AppState {
            store,
            config: Mutex::new(config),
            pending: Arc::new(Mutex::new(HashMap::new())),
        })
        .invoke_handler(tauri::generate_handler![
            list_conversations,
            create_conversation,
            load_messages,
            rename_conversation,
            delete_conversation,
            get_config,
            set_config,
            provider_status,
            set_api_key,
            respond_approval,
            send_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
