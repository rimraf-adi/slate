use slate_core::Engine;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ServerState {
    pub engine: Arc<Mutex<Engine>>,
}

impl ServerState {
    pub fn new(engine: Arc<Mutex<Engine>>) -> Self {
        Self { engine }
    }
}
