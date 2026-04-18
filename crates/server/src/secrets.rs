use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SecretStore {
   path: PathBuf,
   values: Arc<Mutex<BTreeMap<String, String>>>,
}

#[derive(Default, Serialize, Deserialize)]
struct SecretFile {
   values: BTreeMap<String, String>,
}

impl SecretStore {
   pub async fn initialize(path: PathBuf) -> Result<Self> {
      if let Some(parent) = path.parent() {
         tokio::fs::create_dir_all(parent).await?;
      }

      let values = match tokio::fs::read_to_string(&path).await {
         Ok(contents) => serde_json::from_str::<SecretFile>(&contents)
            .map(|file| file.values)
            .unwrap_or_default(),
         Err(_) => BTreeMap::new(),
      };

      Ok(Self {
         path,
         values: Arc::new(Mutex::new(values)),
      })
   }

   pub async fn store(&self, key: String, value: String) -> Result<()> {
      {
         let mut values = self.values.lock().await;
         values.insert(key, value);
      }
      self.save().await
   }

   pub async fn get(&self, key: &str) -> Option<String> {
      self.values.lock().await.get(key).cloned()
   }

   pub async fn remove(&self, key: &str) -> Result<()> {
      {
         let mut values = self.values.lock().await;
         values.remove(key);
      }
      self.save().await
   }

   async fn save(&self) -> Result<()> {
      let values = self.values.lock().await.clone();
      let file = SecretFile { values };
      let contents = serde_json::to_string_pretty(&file)?;
      tokio::fs::write(&self.path, contents).await?;

      #[cfg(unix)]
      {
         use std::os::unix::fs::PermissionsExt;
         let permissions = std::fs::Permissions::from_mode(0o600);
         let _ = std::fs::set_permissions(&self.path, permissions);
      }

      Ok(())
   }
}
