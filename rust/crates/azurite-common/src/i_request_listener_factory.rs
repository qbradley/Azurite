use crate::server_base::RequestListener;

#[allow(non_snake_case)]
pub trait IRequestListenerFactory: Send + Sync {
    fn createRequestListener(&self) -> RequestListener;
}
