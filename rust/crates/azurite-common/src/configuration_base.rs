#[derive(Debug, Clone)]
pub struct ConfigurationBase {
    pub host: String,
    pub port: u16,
}

impl Default for ConfigurationBase {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationBase {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 10000,
        }
    }
}
