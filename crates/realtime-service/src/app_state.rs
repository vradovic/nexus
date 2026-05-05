use crate::connection_registry::ConnectionRegistry;

#[derive(Clone)]
pub struct AppState {
    pub jwt_secret: String,
    pub connection_registry: ConnectionRegistry,
}

impl AppState {
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret,
            connection_registry: ConnectionRegistry::new(),
        }
    }
}
