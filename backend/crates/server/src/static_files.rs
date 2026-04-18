use crate::{
   dev_proxy,
   state::{RelayMode, RelayState},
};
use axum::{
   body::Body,
   extract::{OriginalUri, Query, State, ws::WebSocketUpgrade},
   http::{StatusCode, header},
   response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::{
   path::{Path, PathBuf},
   process::Stdio,
};
use tokio::{fs, process::Command};

#[derive(Deserialize)]
pub struct FileAssetQuery {
   path: String,
}

pub async fn file_asset(Query(query): Query<FileAssetQuery>) -> impl IntoResponse {
   let path = match safe_existing_path(&query.path).await {
      Ok(path) => path,
      Err(_) => return StatusCode::NOT_FOUND.into_response(),
   };
   match fs::read(&path).await {
      Ok(bytes) => {
         let mime = mime_guess::from_path(path).first_or_octet_stream();
         (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            bytes,
         )
            .into_response()
      }
      Err(_) => StatusCode::NOT_FOUND.into_response(),
   }
}

pub async fn serve_app(
   State(state): State<RelayState>,
   request: axum::http::Request<Body>,
) -> Response {
   match &state.mode {
      RelayMode::Development { .. } => serve_dev(state, request).await,
      RelayMode::Production { static_dir } => serve_static(static_dir, request.uri().path()).await,
   }
}

pub async fn serve_dev_websocket(
   State(state): State<RelayState>,
   OriginalUri(original_uri): OriginalUri,
   websocket: WebSocketUpgrade,
) -> Response {
   if !matches!(&state.mode, RelayMode::Development { .. }) {
      return StatusCode::NOT_FOUND.into_response();
   }

   let path_and_query = original_uri
      .path_and_query()
      .map(|value| value.as_str().to_string())
      .unwrap_or_else(|| "/@vite-hmr".to_string());
   websocket.on_upgrade(move |socket| dev_proxy::proxy_websocket(state, socket, path_and_query))
}

async fn serve_dev(state: RelayState, request: axum::http::Request<Body>) -> Response {
   dev_proxy::proxy_http(state, request).await
}

async fn serve_static(static_dir: &PathBuf, request_path: &str) -> Response {
   let static_root = match fs::canonicalize(static_dir).await {
      Ok(path) => path,
      Err(_) => {
         return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Relay static assets were not found. Run `cd app && bun run build` first."
               .as_bytes()
               .to_vec(),
         )
            .into_response();
      }
   };
   let request_path = request_path.trim_start_matches('/');
   let requested = static_root.join(request_path);
   let requested = match fs::canonicalize(&requested).await {
      Ok(path) if path.starts_with(&static_root) && path.is_file() => Some(path),
      _ => None,
   };
   let path = requested.unwrap_or_else(|| static_root.join("index.html"));

   match fs::read(&path).await {
      Ok(bytes) => (
         StatusCode::OK,
         [(
            header::CONTENT_TYPE,
            mime_guess::from_path(path)
               .first_or_text_plain()
               .essence_str()
               .to_string(),
         )],
         bytes,
      )
         .into_response(),
      Err(_) => (
         StatusCode::NOT_FOUND,
         [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
         "Relay static assets were not found. Run `cd app && bun run build` first."
            .as_bytes()
            .to_vec(),
      )
         .into_response(),
   }
}

#[allow(dead_code)]
async fn reveal_item_in_dir(path: PathBuf) -> std::io::Result<()> {
   #[cfg(target_os = "macos")]
   let mut command = {
      let mut command = Command::new("open");
      command.arg("-R").arg(path);
      command
   };

   #[cfg(target_os = "linux")]
   let mut command = {
      let mut command = Command::new("xdg-open");
      command.arg(path.parent().unwrap_or(&path));
      command
   };

   #[cfg(target_os = "windows")]
   let mut command = {
      let mut command = Command::new("explorer");
      command.arg(format!("/select,{}", path.display()));
      command
   };

   command
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()?;
   Ok(())
}

async fn safe_existing_path(path: &str) -> Result<PathBuf, ()> {
   let canonical = fs::canonicalize(path).await.map_err(|_| ())?;
   let roots = allowed_roots().await?;
   if roots.is_empty() {
      return Ok(canonical);
   }

   let allow_symlink_escape = std::env::var("RELAY_ALLOW_SYMLINK_ESCAPE")
      .map(|value| value == "true")
      .unwrap_or(false);
   if allow_symlink_escape || roots.iter().any(|root| canonical.starts_with(root)) {
      Ok(canonical)
   } else {
      Err(())
   }
}

async fn allowed_roots() -> Result<Vec<PathBuf>, ()> {
   let Ok(raw_roots) = std::env::var("RELAY_WORKSPACE_ROOTS") else {
      return Ok(Vec::new());
   };

   let mut roots = Vec::new();
   for root in raw_roots
      .split(',')
      .map(str::trim)
      .filter(|root| !root.is_empty())
   {
      roots.push(fs::canonicalize(Path::new(root)).await.map_err(|_| ())?);
   }
   Ok(roots)
}
