#[derive(Debug, Clone, Default)]
pub struct ServerBase {
    pub is_started: bool,
}

impl ServerBase {
    pub fn new() -> Self {
        Self { is_started: false }
    }
}
