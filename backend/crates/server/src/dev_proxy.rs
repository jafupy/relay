use crate::state::RelayState;
use axum::{
   body::{Body, to_bytes},
   http::{Request, Response, StatusCode, header},
   response::IntoResponse,
};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tokio_tungstenite::{WebSocketStream, client_async};

const MAX_DEV_BODY_BYTES: usize = 64 * 1024 * 1024;

pub async fn proxy_http(state: RelayState, request: Request<Body>) -> Response<Body> {
   let Some(dev_vite) = state.dev_vite.as_ref() else {
      return dev_unavailable("Vite sidecar is not running");
   };

   let stream = match UnixStream::connect(&dev_vite.socket_path).await {
      Ok(stream) => stream,
      Err(error) => return dev_unavailable(&format!("failed to connect to Vite: {}", error)),
   };

   let (parts, body) = request.into_parts();
   let bytes = match to_bytes(body, MAX_DEV_BODY_BYTES).await {
      Ok(bytes) => bytes,
      Err(error) => return dev_unavailable(&format!("failed to read request body: {}", error)),
   };

   let mut builder = Request::builder()
      .method(parts.method)
      .uri(parts.uri)
      .version(parts.version);
   let headers = builder
      .headers_mut()
      .expect("request builder headers exist");
   let mut has_host = false;
   for (name, value) in parts.headers {
      if let Some(name) = name {
         if name == header::HOST {
            has_host = true;
         }
         headers.insert(name, value);
      }
   }
   if !has_host {
      headers.insert(
         header::HOST,
         "127.0.0.1".parse().expect("valid host header"),
      );
   }

   let request = match builder.body(Full::new(bytes)) {
      Ok(request) => request,
      Err(error) => return dev_unavailable(&format!("failed to build Vite request: {}", error)),
   };

   let io = TokioIo::new(stream);
   let (mut sender, connection) = match http1::handshake(io).await {
      Ok(parts) => parts,
      Err(error) => return dev_unavailable(&format!("failed to open Vite connection: {}", error)),
   };
   tokio::spawn(async move {
      if let Err(error) = connection.await {
         eprintln!("Vite proxy connection error: {}", error);
      }
   });

   let response = match sender.send_request(request).await {
      Ok(response) => response,
      Err(error) => return dev_unavailable(&format!("Vite request failed: {}", error)),
   };
   let (parts, body) = response.into_parts();
   let bytes = match body.collect().await {
      Ok(body) => body.to_bytes(),
      Err(error) => return dev_unavailable(&format!("failed to read Vite response: {}", error)),
   };
   Response::from_parts(parts, Body::from(bytes))
}

pub async fn proxy_websocket(
   state: RelayState,
   relay_socket: axum::extract::ws::WebSocket,
   path_and_query: String,
) {
   let Some(dev_vite) = state.dev_vite.as_ref() else {
      return;
   };
   let Ok(stream) = UnixStream::connect(&dev_vite.socket_path).await else {
      return;
   };
   let url = format!("ws://relay-vite{}", path_and_query);
   let Ok((vite_socket, _)) = client_async(url, stream).await else {
      return;
   };
   bridge_websockets(relay_socket, vite_socket).await;
}

async fn bridge_websockets(
   relay_socket: axum::extract::ws::WebSocket,
   vite_socket: WebSocketStream<UnixStream>,
) {
   let (mut relay_tx, mut relay_rx) = relay_socket.split();
   let (mut vite_tx, mut vite_rx) = vite_socket.split();

   let relay_to_vite = async {
      while let Some(Ok(message)) = relay_rx.next().await {
         let message = match message {
            axum::extract::ws::Message::Text(value) => {
               tokio_tungstenite::tungstenite::Message::Text(value.to_string().into())
            }
            axum::extract::ws::Message::Binary(value) => {
               tokio_tungstenite::tungstenite::Message::Binary(value)
            }
            axum::extract::ws::Message::Ping(value) => {
               tokio_tungstenite::tungstenite::Message::Ping(value)
            }
            axum::extract::ws::Message::Pong(value) => {
               tokio_tungstenite::tungstenite::Message::Pong(value)
            }
            axum::extract::ws::Message::Close(frame) => {
               let close =
                  frame.map(
                     |frame| tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: frame.code.into(),
                        reason: frame.reason.to_string().into(),
                     },
                  );
               tokio_tungstenite::tungstenite::Message::Close(close)
            }
         };
         if vite_tx.send(message).await.is_err() {
            break;
         }
      }
   };

   let vite_to_relay = async {
      while let Some(Ok(message)) = vite_rx.next().await {
         let message = match message {
            tokio_tungstenite::tungstenite::Message::Text(value) => {
               axum::extract::ws::Message::Text(value.to_string().into())
            }
            tokio_tungstenite::tungstenite::Message::Binary(value) => {
               axum::extract::ws::Message::Binary(value)
            }
            tokio_tungstenite::tungstenite::Message::Ping(value) => {
               axum::extract::ws::Message::Ping(value)
            }
            tokio_tungstenite::tungstenite::Message::Pong(value) => {
               axum::extract::ws::Message::Pong(value)
            }
            tokio_tungstenite::tungstenite::Message::Close(frame) => {
               let close = frame.map(|frame| axum::extract::ws::CloseFrame {
                  code: frame.code.into(),
                  reason: frame.reason.to_string().into(),
               });
               axum::extract::ws::Message::Close(close)
            }
            tokio_tungstenite::tungstenite::Message::Frame(_) => continue,
         };
         if relay_tx.send(message).await.is_err() {
            break;
         }
      }
   };

   tokio::select! {
      _ = relay_to_vite => {}
      _ = vite_to_relay => {}
   }
}

fn dev_unavailable(message: &str) -> Response<Body> {
   (
      StatusCode::BAD_GATEWAY,
      [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
      Bytes::from(message.to_string()),
   )
      .into_response()
}
