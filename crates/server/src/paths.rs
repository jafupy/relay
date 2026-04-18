use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Clone)]
pub struct RelayPaths {
   pub config_dir: PathBuf,
}

impl RelayPaths {
   pub fn resolve() -> Result<Self> {
      let config_dir = if let Ok(path) = std::env::var("RELAY_CONFIG_DIR") {
         PathBuf::from(path)
      } else if let Ok(path) = std::env::var("XDG_CONFIG_HOME") {
         PathBuf::from(path).join("relay")
      } else {
         let home = std::env::var("HOME").context("HOME is not set")?;
         PathBuf::from(home).join(".config").join("relay")
      };

      std::fs::create_dir_all(&config_dir)
         .with_context(|| format!("failed to create {}", config_dir.display()))?;

      Ok(Self { config_dir })
   }

   pub fn auth_dir(&self) -> PathBuf {
      self.config_dir.join("auth")
   }

   pub fn passkeys_file(&self) -> PathBuf {
      self.auth_dir().join("passkeys.json")
   }

   pub fn data_dir(&self) -> PathBuf {
      self.config_dir.join("data")
   }

   pub fn secrets_file(&self) -> PathBuf {
      self.config_dir.join("secrets.json")
   }

   pub fn chat_history_db(&self) -> PathBuf {
      self.data_dir().join("chat_history.db")
   }
}
