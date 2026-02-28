use crate::document::DocumentStore;

/// Central server state holding all open documents and status.
pub struct ServerState {
    pub documents: DocumentStore,
    pub initialized: bool,
    pub shutdown_requested: bool,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            documents: DocumentStore::new(),
            initialized: false,
            shutdown_requested: false,
        }
    }
}
