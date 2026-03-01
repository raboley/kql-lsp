use crate::document::DocumentStore;
use crate::schema::SchemaStore;

/// Central server state holding all open documents and status.
pub struct ServerState {
    pub documents: DocumentStore,
    pub schema: SchemaStore,
    pub initialized: bool,
    pub shutdown_requested: bool,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            documents: DocumentStore::new(),
            schema: SchemaStore::new(),
            initialized: false,
            shutdown_requested: false,
        }
    }
}
