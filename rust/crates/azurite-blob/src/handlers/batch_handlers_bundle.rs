use std::{str::FromStr, sync::Arc};

use azurite_common::{i_account_data_store::IAccountDataStore, models::OAuthLevel};

use crate::generated::{
    context::Context,
    handlers::{
        IAppendBlobHandler, IBlobHandler, IBlockBlobHandler, IContainerHandler, IHandlers,
        IPageBlobHandler, IServiceHandler,
    },
    i_request::IRequest,
};

use super::{
    append_blob_handler::AppendBlobHandler, base_handler::BaseHandler,
    blob_batch_handler::BlobBatchHandler, blob_batch_sub_request::BlobBatchSubRequest,
    blob_handler::BlobHandler, block_blob_handler::BlockBlobHandler,
    container_handler::ContainerHandler, page_blob_handler::PageBlobHandler,
    page_blob_ranges_manager::PageBlobRangesManager, service_handler::ServiceHandler,
};

#[derive(Clone)]
pub struct BatchHandlersBundle {
    service: Arc<ServiceHandler>,
    container: Arc<ContainerHandler>,
    blob: Arc<BlobHandler>,
    page: Arc<PageBlobHandler>,
    append: Arc<AppendBlobHandler>,
    block: Arc<BlockBlobHandler>,
}

impl IHandlers for BatchHandlersBundle {
    fn serviceHandler(&self) -> &(dyn IServiceHandler + Send + Sync) {
        self.service.as_ref()
    }

    fn containerHandler(&self) -> &(dyn IContainerHandler + Send + Sync) {
        self.container.as_ref()
    }

    fn blobHandler(&self) -> &(dyn IBlobHandler + Send + Sync) {
        self.blob.as_ref()
    }

    fn pageBlobHandler(&self) -> &(dyn IPageBlobHandler + Send + Sync) {
        self.page.as_ref()
    }

    fn appendBlobHandler(&self) -> &(dyn IAppendBlobHandler + Send + Sync) {
        self.append.as_ref()
    }

    fn blockBlobHandler(&self) -> &(dyn IBlockBlobHandler + Send + Sync) {
        self.block.as_ref()
    }
}

pub fn create_blob_batch_handler(
    service: ServiceHandler,
    container: ContainerHandler,
    base: BaseHandler,
    account_data_store: Arc<dyn IAccountDataStore + Send + Sync>,
    oauth: Option<String>,
    disable_product_style: Option<bool>,
) -> BlobBatchHandler<BatchHandlersBundle> {
    let ranges_manager = Arc::new(PageBlobRangesManager::new());
    let handlers = Arc::new(BatchHandlersBundle {
        service: Arc::new(service),
        container: Arc::new(container),
        blob: Arc::new(BlobHandler::new(base.clone(), ranges_manager.clone())),
        page: Arc::new(PageBlobHandler::new(base.clone(), ranges_manager)),
        append: Arc::new(AppendBlobHandler::new(base.clone())),
        block: Arc::new(BlockBlobHandler::new(base.clone())),
    });

    BlobBatchHandler::new(
        account_data_store,
        oauth.and_then(|value| OAuthLevel::from_str(&value).ok()),
        base.metadataStore.clone(),
        base.extentStore.clone(),
        base.logger.clone(),
        base.loose,
        disable_product_style,
        handlers,
    )
}

pub fn create_batch_request(context: &Context) -> Option<BlobBatchSubRequest> {
    let request = context.request()?;
    Some(BlobBatchSubRequest::new(
        0,
        request.getUrl(),
        request.getMethod(),
        String::from("HTTP/1.1"),
        request.getHeaders(),
    ))
}

pub fn parse_batch_boundary(multipart_content_type: &str) -> Option<String> {
    multipart_content_type
        .split(';')
        .find_map(|segment| {
            let segment = segment.trim();
            segment
                .strip_prefix("boundary=")
                .or_else(|| segment.strip_prefix("Boundary="))
                .map(|value| value.trim_matches('"').to_string())
        })
        .filter(|value| !value.is_empty())
}
