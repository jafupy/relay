use crate::events::EventHub;
use anyhow::{Result, anyhow};
use portable_pty::{Child, CommandBuilder, PtyPair, PtySize};
use serde::{Deserialize, Serialize};
use std::{
   collections::HashMap,
   io::{Read, Write},
   path::Path,
   sync::{Arc, Mutex},
   thread,
};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct TerminalConfig {
   pub working_directory: Option<String>,
   pub shell: Option<String>,
   pub environment: Option<HashMap<String, String>>,
   pub command: Option<String>,
   pub args: Option<Vec<String>>,
   #[serde(default = "default_rows")]
   pub rows: u16,
   #[serde(default = "default_cols")]
   pub cols: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shell {
   pub id: String,
   pub name: String,
   pub exec_win: Option<String>,
   pub exec_unix: Option<String>,
}

pub struct TerminalManager {
   connections: Mutex<HashMap<String, TerminalConnection>>,
   events: EventHub,
}

struct TerminalConnection {
   id: String,
   pty_pair: PtyPair,
   writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
   child: Arc<Mutex<Option<Box<dyn Child + Send + Sync>>>>,
   events: EventHub,
}

impl TerminalManager {
   pub fn new(events: EventHub) -> Self {
      Self {
         connections: Mutex::new(HashMap::new()),
         events,
      }
   }

   pub fn create_terminal(&self, config: TerminalConfig) -> Result<String> {
      let id = Uuid::new_v4().to_string();
      let connection = TerminalConnection::new(id.clone(), config, self.events.clone())?;
      connection.start_reader_thread()?;
      self
         .connections
         .lock()
         .map_err(|_| anyhow!("terminal manager lock was poisoned"))?
         .insert(id.clone(), connection);
      Ok(id)
   }

   pub fn write_to_terminal(&self, id: &str, data: &str) -> Result<()> {
      let connections = self
         .connections
         .lock()
         .map_err(|_| anyhow!("terminal manager lock was poisoned"))?;
      let connection = connections
         .get(id)
         .ok_or_else(|| anyhow!("terminal connection not found"))?;
      connection.write(data)
   }

   pub fn resize_terminal(&self, id: &str, rows: u16, cols: u16) -> Result<()> {
      let connections = self
         .connections
         .lock()
         .map_err(|_| anyhow!("terminal manager lock was poisoned"))?;
      let connection = connections
         .get(id)
         .ok_or_else(|| anyhow!("terminal connection not found"))?;
      connection.resize(rows, cols)
   }

   pub fn close_terminal(&self, id: &str) -> Result<()> {
      let connection = self
         .connections
         .lock()
         .map_err(|_| anyhow!("terminal manager lock was poisoned"))?
         .remove(id);
      if let Some(connection) = connection {
         let _ = connection.kill();
      }
      Ok(())
   }
}

impl TerminalConnection {
   fn new(id: String, config: TerminalConfig, events: EventHub) -> Result<Self> {
      let pty_system = portable_pty::native_pty_system();
      let pty_pair = pty_system.openpty(PtySize {
         rows: config.rows.max(1),
         cols: config.cols.max(1),
         pixel_width: 0,
         pixel_height: 0,
      })?;
      let command = build_command(&config)?;
      let child = pty_pair.slave.spawn_command(command)?;
      let writer = Arc::new(Mutex::new(Some(pty_pair.master.take_writer()?)));
      let child = Arc::new(Mutex::new(Some(child)));

      Ok(Self {
         id,
         pty_pair,
         writer,
         child,
         events,
      })
   }

   fn start_reader_thread(&self) -> Result<()> {
      let id = self.id.clone();
      let events = self.events.clone();
      let child = self.child.clone();
      let mut reader = self.pty_pair.master.try_clone_reader()?;

      thread::spawn(move || {
         let mut buffer = vec![0u8; 65536];
         let mut incomplete_utf8: Vec<u8> = Vec::new();

         loop {
            match reader.read(&mut buffer) {
               Ok(0) => {
                  let mut exit_code = None;
                  let mut signal = None;
                  if let Ok(mut child_guard) = child.lock()
                     && let Some(child) = child_guard.as_mut()
                  {
                     let status = child
                        .try_wait()
                        .ok()
                        .flatten()
                        .or_else(|| child.wait().ok());
                     if let Some(status) = status {
                        exit_code = Some(status.exit_code());
                        signal = status.signal().map(str::to_string);
                     }
                  }

                  events.emit(
                     format!("pty-exit-{}", id),
                     serde_json::json!({ "exitCode": exit_code, "signal": signal }),
                  );
                  events.emit(format!("pty-closed-{}", id), serde_json::json!({}));
                  break;
               }
               Ok(n) => {
                  let mut data = if incomplete_utf8.is_empty() {
                     buffer[..n].to_vec()
                  } else {
                     let mut combined = std::mem::take(&mut incomplete_utf8);
                     combined.extend_from_slice(&buffer[..n]);
                     combined
                  };
                  let valid_up_to = find_utf8_boundary(&data);
                  if valid_up_to < data.len() {
                     incomplete_utf8 = data[valid_up_to..].to_vec();
                     data.truncate(valid_up_to);
                  }
                  if !data.is_empty() {
                     events.emit(
                        format!("pty-output-{}", id),
                        serde_json::json!({ "data": String::from_utf8_lossy(&data) }),
                     );
                  }
               }
               Err(error) => {
                  events.emit(
                     format!("pty-error-{}", id),
                     serde_json::json!({ "error": error.to_string() }),
                  );
                  events.emit(format!("pty-closed-{}", id), serde_json::json!({}));
                  break;
               }
            }
         }
      });

      Ok(())
   }

   fn write(&self, data: &str) -> Result<()> {
      let mut writer = self
         .writer
         .lock()
         .map_err(|_| anyhow!("terminal writer lock was poisoned"))?;
      let writer = writer
         .as_mut()
         .ok_or_else(|| anyhow!("terminal writer is closed"))?;
      writer.write_all(data.as_bytes())?;
      writer.flush()?;
      Ok(())
   }

   fn resize(&self, rows: u16, cols: u16) -> Result<()> {
      self.pty_pair.master.resize(PtySize {
         rows: rows.max(1),
         cols: cols.max(1),
         pixel_width: 0,
         pixel_height: 0,
      })?;
      Ok(())
   }

   fn kill(&self) -> Result<()> {
      if let Some(mut child) = self
         .child
         .lock()
         .map_err(|_| anyhow!("terminal child lock was poisoned"))?
         .take()
      {
         let _ = child.kill();
      }
      Ok(())
   }
}

pub fn list_shells() -> Vec<Shell> {
   shell_candidates()
      .into_iter()
      .filter(|shell| {
         let path = if cfg!(target_os = "windows") {
            shell.exec_win.as_deref()
         } else {
            shell.exec_unix.as_deref()
         };
         path.map(|path| Path::new(path).exists()).unwrap_or(false)
      })
      .collect()
}

fn build_command(config: &TerminalConfig) -> Result<CommandBuilder> {
   let (mut command, shell_path) = if let Some(command) = &config.command {
      let mut builder = CommandBuilder::new(command);
      if let Some(args) = &config.args {
         builder.args(args);
      }
      (builder, None)
   } else {
      let shell_path = config
         .shell
         .as_deref()
         .and_then(resolve_shell_by_id)
         .unwrap_or_else(default_shell);
      (CommandBuilder::new(&shell_path), Some(shell_path))
   };

   if let Some(working_directory) = &config.working_directory {
      command.cwd(working_directory);
   }

   for (key, value) in std::env::vars() {
      command.env(key, value);
   }
   command.env("TERM", "xterm-256color");
   command.env("COLORTERM", "truecolor");
   command.env("TERM_PROGRAM", "relay");
   command.env("FORCE_COLOR", "1");
   command.env("CLICOLOR", "1");
   command.env("CLICOLOR_FORCE", "1");
   if let Some(shell_path) = shell_path {
      command.env("SHELL", shell_path);
   }
   if let Some(environment) = &config.environment {
      for (key, value) in environment {
         command.env(key, value);
      }
   }

   Ok(command)
}

fn resolve_shell_by_id(id: &str) -> Option<String> {
   list_shells().into_iter().find_map(|shell| {
      if shell.id != id {
         return None;
      }
      if cfg!(target_os = "windows") {
         shell.exec_win
      } else {
         shell.exec_unix
      }
   })
}

fn shell_candidates() -> Vec<Shell> {
   if cfg!(target_os = "windows") {
      vec![
         shell("cmd", "Command Prompt", Some("cmd.exe"), None),
         shell(
            "powershell",
            "Windows PowerShell",
            Some("powershell.exe"),
            None,
         ),
         shell("pwsh", "PowerShell Core", Some("pwsh.exe"), None),
         shell("nu", "Nushell", Some("nu.exe"), None),
         shell("wsl", "Windows Subsystem for Linux", Some("wsl.exe"), None),
         shell("bash", "Git Bash", Some("bash.exe"), None),
      ]
   } else {
      vec![
         shell("bash", "Bash", None, Some("bash")),
         shell("zsh", "Zsh", None, Some("zsh")),
         shell("fish", "Fish", None, Some("fish")),
         shell("nu", "Nushell", None, Some("nu")),
      ]
   }
}

fn shell(id: &str, name: &str, exec_win: Option<&str>, exec_unix: Option<&str>) -> Shell {
   Shell {
      id: id.to_string(),
      name: name.to_string(),
      exec_win: exec_win.and_then(resolve_exe),
      exec_unix: exec_unix.and_then(resolve_exe),
   }
}

fn resolve_exe(exe: &str) -> Option<String> {
   std::env::var_os("PATH").and_then(|paths| {
      std::env::split_paths(&paths).find_map(|dir| {
         let full_path = dir.join(exe);
         if full_path.exists() {
            Some(full_path.to_string_lossy().into_owned())
         } else {
            None
         }
      })
   })
}

fn default_shell() -> String {
   if cfg!(target_os = "windows") {
      "cmd.exe".to_string()
   } else {
      std::env::var("SHELL").unwrap_or_else(|_| {
         if Path::new("/bin/zsh").exists() {
            "/bin/zsh".to_string()
         } else if Path::new("/bin/bash").exists() {
            "/bin/bash".to_string()
         } else {
            "/bin/sh".to_string()
         }
      })
   }
}

fn find_utf8_boundary(bytes: &[u8]) -> usize {
   for index in (0..=bytes.len()).rev() {
      if std::str::from_utf8(&bytes[..index]).is_ok() {
         return index;
      }
   }
   0
}

fn default_rows() -> u16 {
   24
}

fn default_cols() -> u16 {
   80
}
