mod auth;
mod chat_history;
mod dev_proxy;
mod dev_vite;
mod events;
mod paths;
mod router;
mod rpc;
mod secrets;
mod state;
mod static_files;
mod terminal;
mod webauthn;

use anyhow::Context;
use state::{RelayMode, RelayState};
use std::{
   net::{IpAddr, SocketAddr},
   path::PathBuf,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
   let args = Args::parse();
   let state = RelayState::initialize(args.mode).await?;
   let app = router::build_router(state);
   let addr = SocketAddr::from((args.host, args.port));

   println!("Relay listening on http://{}", addr);

   let listener = tokio::net::TcpListener::bind(addr)
      .await
      .with_context(|| format!("failed to bind {}", addr))?;
   axum::serve(listener, app).await?;
   Ok(())
}

struct Args {
   mode: RelayMode,
   host: IpAddr,
   port: u16,
}

impl Args {
   fn parse() -> Self {
      let mut mode = RelayMode::Production {
         static_dir: PathBuf::from("dist"),
      };
      let mut port = std::env::var("RELAY_PORT")
         .ok()
         .and_then(|value| value.parse::<u16>().ok())
         .unwrap_or(1420);
      let mut host = std::env::var("RELAY_HOST")
         .ok()
         .and_then(|value| value.parse::<IpAddr>().ok())
         .unwrap_or_else(|| "127.0.0.1".parse().expect("valid loopback address"));

      let mut args = std::env::args().skip(1).peekable();
      while let Some(arg) = args.next() {
         match arg.as_str() {
            "--dev" => {
               mode = RelayMode::Development;
            }
            "--static-dir" => {
               if let Some(path) = args.next() {
                  mode = RelayMode::Production {
                     static_dir: PathBuf::from(path),
                  };
               }
            }
            "--port" => {
               if let Some(value) = args.next().and_then(|value| value.parse::<u16>().ok()) {
                  port = value;
               }
            }
            "--host" => {
               if let Some(value) = args.next().and_then(|value| value.parse::<IpAddr>().ok()) {
                  host = value;
               }
            }
            _ => {}
         }
      }

      Self { mode, host, port }
   }
}
