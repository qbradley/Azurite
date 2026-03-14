use super::{IServiceHandler, ITableHandler};

#[allow(non_snake_case)]
pub trait IHandlers: Send + Sync {
    fn serviceHandler(&self) -> &(dyn IServiceHandler + Send + Sync);
    fn tableHandler(&self) -> &(dyn ITableHandler + Send + Sync);
}
