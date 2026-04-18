use anyhow::{Context, Result, anyhow};
use std::{
   path::PathBuf,
   process::Stdio,
   sync::{Arc, Mutex},
   time::Duration,
};
use tokio::{
   io::{AsyncBufReadExt, BufReader},
   process::{Child, Command},
};

pub struct DevVite {
   pub socket_path: PathBuf,
   child: Arc<Mutex<Child>>,
}

impl DevVite {
   pub async fn start(app_dir: PathBuf) -> Result<Self> {
      let socket_path =
         std::env::temp_dir().join(format!("relay-vite-{}.sock", std::process::id()));
      let _ = tokio::fs::remove_file(&socket_path).await;

      let mut child = Command::new("bun")
         .current_dir(&app_dir)
         .arg("scripts/relay-vite-dev-server.mjs")
         .arg("--socket")
         .arg(&socket_path)
         .stdin(Stdio::null())
         .stdout(Stdio::piped())
         .stderr(Stdio::inherit())
         .spawn()
         .with_context(|| format!("failed to start Vite sidecar from {}", app_dir.display()))?;

      let stdout = child
         .stdout
         .take()
         .ok_or_else(|| anyhow!("failed to capture Vite sidecar stdout"))?;
      let mut lines = BufReader::new(stdout).lines();

      let mut ready = false;
      for _ in 0..100 {
         tokio::select! {
            line = lines.next_line() => {
               if let Some(line) = line? {
                  println!("{}", line);
                  if line.contains("socketPath") {
                     ready = true;
                     break;
                  }
               }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
               if tokio::fs::metadata(&socket_path).await.is_ok() {
                  ready = true;
                  break;
               }
            }
         }
      }

      tokio::spawn(async move {
         while let Ok(Some(line)) = lines.next_line().await {
            println!("{}", line);
         }
      });

      if !ready {
         let _ = child.start_kill();
         return Err(anyhow!(
            "Vite sidecar did not create {}",
            socket_path.display()
         ));
      }

      println!(
         "Relay dev assets are served through {}",
         socket_path.display()
      );
      Ok(Self {
         socket_path,
         child: Arc::new(Mutex::new(child)),
      })
   }
}

impl Drop for DevVite {
   fn drop(&mut self) {
      if let Ok(mut child) = self.child.lock() {
         let _ = child.start_kill();
      }
      let _ = std::fs::remove_file(&self.socket_path);
   }
}
