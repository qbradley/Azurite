pub trait IAuthenticationContext {
    fn account(&self) -> Option<String>;
}
