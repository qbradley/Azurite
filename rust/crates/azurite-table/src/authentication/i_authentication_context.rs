pub trait IAuthenticationContext: Send + Sync {
    fn account(&self) -> Option<String>;
}
