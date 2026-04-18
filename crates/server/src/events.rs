use crate::state::RelayState;
use axum::{
   extract::{
      State,
      ws::{Message, WebSocket, WebSocketUpgrade},
   },
   response::Response,
};
use serde::Serialize;
use serde_json::Value;
use std::{
   collections::HashMap,
   sync::{
      Arc, Mutex,
      atomic::{AtomicU64, Ordering},
   },
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct EventHub {
   tx: broadcast::Sender<String>,
   listeners: Arc<Mutex<HashMap<u64, EventListener>>>,
   next_listener_id: Arc<AtomicU64>,
}

pub type EventListenerCallback = Arc<dyn Fn(Value) + Send + Sync + 'static>;

struct EventListener {
   event: String,
   callback: EventListenerCallback,
}

#[derive(Serialize)]
pub struct RelayEvent<T> {
   event: String,
   payload: T,
}

impl EventHub {
   pub fn new() -> Self {
      let (tx, _) = broadcast::channel(512);
      Self {
         tx,
         listeners: Arc::new(Mutex::new(HashMap::new())),
         next_listener_id: Arc::new(AtomicU64::new(1)),
      }
   }

   pub fn emit<T: Serialize>(&self, event: impl Into<String>, payload: T) {
      let event = event.into();
      let payload = match serde_json::to_value(payload) {
         Ok(payload) => payload,
         Err(error) => {
            eprintln!("Failed to serialize event payload for {event}: {error}");
            return;
         }
      };
      self.notify_listeners(&event, payload.clone());

      let payload = RelayEvent { event, payload };
      if let Ok(message) = serde_json::to_string(&payload) {
         let _ = self.tx.send(message);
      }
   }

   pub fn listen(&self, event: String, callback: EventListenerCallback) -> u64 {
      let listener_id = self.next_listener_id.fetch_add(1, Ordering::Relaxed);
      if let Ok(mut listeners) = self.listeners.lock() {
         listeners.insert(listener_id, EventListener { event, callback });
      }
      listener_id
   }

   pub fn unlisten(&self, listener_id: u64) {
      if let Ok(mut listeners) = self.listeners.lock() {
         listeners.remove(&listener_id);
      }
   }

   fn notify_listeners(&self, event: &str, payload: Value) {
      let callbacks = if let Ok(listeners) = self.listeners.lock() {
         listeners
            .values()
            .filter(|listener| listener.event == event)
            .map(|listener| listener.callback.clone())
            .collect::<Vec<_>>()
      } else {
         Vec::new()
      };

      for callback in callbacks {
         callback(payload.clone());
      }
   }

   pub fn subscribe(&self) -> broadcast::Receiver<String> {
      self.tx.subscribe()
   }
}

pub async fn events_socket(State(state): State<RelayState>, ws: WebSocketUpgrade) -> Response {
   ws.on_upgrade(move |socket| stream_events(socket, state.events.subscribe()))
}

async fn stream_events(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
   while let Ok(message) = rx.recv().await {
      if socket.send(Message::Text(message.into())).await.is_err() {
         break;
      }
   }
}
