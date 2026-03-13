#[allow(non_snake_case)]
pub trait ILogger: Send + Sync {
    fn error(&self, message: &str, contextID: Option<&str>);
    fn warn(&self, message: &str, contextID: Option<&str>);
    fn info(&self, message: &str, contextID: Option<&str>);
    fn verbose(&self, message: &str, contextID: Option<&str>);
    fn debug(&self, message: &str, contextID: Option<&str>);
}
