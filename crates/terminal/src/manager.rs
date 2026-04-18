use crate::{TerminalEventSink, config::TerminalConfig, connection::TerminalConnection};
use anyhow::{Result, anyhow};
use std::{
   collections::HashMap,
   sync::{Arc, Mutex},
};
use uuid::Uuid;

pub struct TerminalManager {
   connections: Arc<Mutex<HashMap<String, TerminalConnection>>>,
   event_sink: Arc<dyn TerminalEventSink>,
}

impl TerminalManager {
   pub fn new(event_sink: Arc<dyn TerminalEventSink>) -> Self {
      Self {
         connections: Arc::new(Mutex::new(HashMap::new())),
         event_sink,
      }
   }

   pub fn create_terminal(&self, config: TerminalConfig) -> Result<String> {
      let id = Uuid::new_v4().to_string();
      let connection = TerminalConnection::new(id.clone(), config, self.event_sink.clone())?;

      // Start the reader thread
      connection.start_reader_thread();

      // Store the connection
      let mut connections = self.connections.lock().unwrap();
      connections.insert(id.clone(), connection);

      Ok(id)
   }

   pub fn write_to_terminal(&self, id: &str, data: &str) -> Result<()> {
      let connections = self.connections.lock().unwrap();
      if let Some(connection) = connections.get(id) {
         connection.write(data)
      } else {
         Err(anyhow!("Terminal connection not found"))
      }
   }

   pub fn resize_terminal(&self, id: &str, rows: u16, cols: u16) -> Result<()> {
      let connections = self.connections.lock().unwrap();
      if let Some(connection) = connections.get(id) {
         connection.resize(rows, cols)
      } else {
         Err(anyhow!("Terminal connection not found"))
      }
   }

   pub fn close_terminal(&self, id: &str) -> Result<()> {
      let mut connections = self.connections.lock().unwrap();
      if let Some(connection) = connections.remove(id)
         && let Err(e) = connection.kill()
      {
         log::debug!("Terminal {} kill returned error: {}", id, e);
      }
      Ok(())
   }

   pub fn kill_terminal(&self, id: &str) -> Result<()> {
      let connections = self.connections.lock().unwrap();
      if let Some(connection) = connections.get(id) {
         connection.kill()
      } else {
         Err(anyhow!("Terminal connection not found"))
      }
   }
}
