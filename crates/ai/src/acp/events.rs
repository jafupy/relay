use serde::Serialize;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

pub type AcpListenerId = u64;
pub type AcpListenerCallback = Arc<dyn Fn(Value) + Send + Sync + 'static>;

pub trait AcpEventSink: Send + Sync {
   fn emit_json(&self, event: &str, payload: Value);

   fn listen_json(&self, event: &str, callback: AcpListenerCallback) -> AcpListenerId {
      let _ = event;
      let _ = callback;
      0
   }

   fn unlisten(&self, listener_id: AcpListenerId) {
      let _ = listener_id;
   }

   fn data_dir(&self) -> Option<PathBuf> {
      None
   }
}

pub fn emit<T: Serialize>(
   event_sink: &dyn AcpEventSink,
   event: &str,
   payload: T,
) -> Result<(), serde_json::Error> {
   event_sink.emit_json(event, serde_json::to_value(payload)?);
   Ok(())
}
