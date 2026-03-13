#[derive(Debug, Clone, Default)]
pub struct Environment {
    pub args: Vec<String>,
}

impl Environment {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}
