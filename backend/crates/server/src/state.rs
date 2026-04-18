use crate::{
   auth::AuthStore, chat_history::ChatHistoryRepository, dev_vite::DevVite, events::EventHub,
   paths::RelayPaths, secrets::SecretStore, webauthn::PasskeyStore,
};
use anyhow::Result;
use relay_ai::AcpEventSink;
use relay_lsp::client::LspEventSink;
use relay_project::{FileChangeEmitter, FileChangeEvent, FileWatcher};
use relay_remote::RemoteEventSink;
use relay_terminal::TerminalEventSink;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RelayState {
   pub mode: RelayMode,
   pub auth: Arc<AuthStore>,
   pub passkeys: Arc<PasskeyStore>,
   pub file_watcher: Arc<FileWatcher>,
   pub terminal_manager: Arc<crate::terminal::TerminalManager>,
   pub file_clipboard: Arc<Mutex<Option<crate::rpc::FileClipboardState>>>,
   pub events: EventHub,
   pub secrets: SecretStore,
   pub database: Arc<relay_database::ConnectionManager>,
   pub chat_history: Arc<ChatHistoryRepository>,
   pub lsp_manager: Arc<relay_lsp::LspManager>,
   pub acp_bridge: Arc<Mutex<relay_ai::AcpAgentBridge>>,
   pub event_sink: Arc<ServerEventSink>,
   pub data_dir: PathBuf,
   pub dev_vite: Option<Arc<DevVite>>,
}

#[derive(Clone)]
pub enum RelayMode {
   Development { app_dir: PathBuf },
   Production { static_dir: PathBuf },
}

impl RelayState {
   pub async fn initialize(mode: RelayMode) -> Result<Self> {
      let paths = RelayPaths::resolve()?;
      let events = EventHub::new();
      let event_sink = Arc::new(ServerEventSink {
         events: events.clone(),
         data_dir: paths.data_dir(),
      });
      let auth = Arc::new(AuthStore::initialize(paths.auth_dir()).await?);
      let passkeys = Arc::new(PasskeyStore::initialize(paths.passkeys_file()).await?);
      let secrets = SecretStore::initialize(paths.secrets_file()).await?;
      let database = Arc::new(relay_database::ConnectionManager::new());
      let chat_history = Arc::new(ChatHistoryRepository::new(paths.chat_history_db()));
      let lsp_manager = Arc::new(relay_lsp::LspManager::new(
         event_sink.clone(),
         paths.data_dir(),
      ));
      let acp_terminal_manager = Arc::new(relay_terminal::TerminalManager::new(event_sink.clone()));
      let acp_bridge = Arc::new(Mutex::new(relay_ai::AcpAgentBridge::new(
         event_sink.clone(),
         acp_terminal_manager,
      )));
      let file_watcher = Arc::new(FileWatcher::new(Arc::new(ServerFileChangeEmitter {
         events: events.clone(),
      })));
      let terminal_manager = Arc::new(crate::terminal::TerminalManager::new(events.clone()));
      let dev_vite = match &mode {
         RelayMode::Development { app_dir } => {
            Some(Arc::new(DevVite::start(app_dir.clone()).await?))
         }
         RelayMode::Production { .. } => None,
      };

      Ok(Self {
         mode,
         auth,
         passkeys,
         file_watcher,
         terminal_manager,
         file_clipboard: Arc::new(Mutex::new(None)),
         events,
         secrets,
         database,
         chat_history,
         lsp_manager,
         acp_bridge,
         event_sink,
         data_dir: paths.data_dir(),
         dev_vite,
      })
   }
}

pub struct ServerEventSink {
   events: EventHub,
   data_dir: PathBuf,
}

impl LspEventSink for ServerEventSink {
   fn emit_json(&self, event: &str, payload: Value) {
      self.events.emit(event, payload);
   }

   fn data_dir(&self) -> Option<PathBuf> {
      Some(self.data_dir.clone())
   }
}

impl RemoteEventSink for ServerEventSink {
   fn emit_json(&self, event: &str, payload: Value) {
      self.events.emit(event, payload);
   }
}

impl TerminalEventSink for ServerEventSink {
   fn emit_json(&self, event: &str, payload: Value) {
      self.events.emit(event, payload);
   }
}

impl AcpEventSink for ServerEventSink {
   fn emit_json(&self, event: &str, payload: Value) {
      self.events.emit(event, payload);
   }

   fn listen_json(
      &self,
      event: &str,
      callback: relay_ai::AcpListenerCallback,
   ) -> relay_ai::AcpListenerId {
      self.events.listen(event.to_string(), callback)
   }

   fn unlisten(&self, listener_id: relay_ai::AcpListenerId) {
      self.events.unlisten(listener_id);
   }

   fn data_dir(&self) -> Option<PathBuf> {
      Some(self.data_dir.clone())
   }
}

struct ServerFileChangeEmitter {
   events: EventHub,
}

impl FileChangeEmitter for ServerFileChangeEmitter {
   fn emit_file_change(&self, event: &FileChangeEvent) {
      self.events.emit("file-changed", event);
   }
}
