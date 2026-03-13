use super::{
    IAppendBlobHandler, IBlobHandler, IBlockBlobHandler, IContainerHandler, IPageBlobHandler,
    IServiceHandler,
};

#[allow(non_snake_case)]
pub trait IHandlers: Send + Sync {
    fn serviceHandler(&self) -> &(dyn IServiceHandler + Send + Sync);
    fn containerHandler(&self) -> &(dyn IContainerHandler + Send + Sync);
    fn blobHandler(&self) -> &(dyn IBlobHandler + Send + Sync);
    fn pageBlobHandler(&self) -> &(dyn IPageBlobHandler + Send + Sync);
    fn appendBlobHandler(&self) -> &(dyn IAppendBlobHandler + Send + Sync);
    fn blockBlobHandler(&self) -> &(dyn IBlockBlobHandler + Send + Sync);
}
