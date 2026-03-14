pub mod append_blob_handler;
pub mod base_handler;
pub mod blob_batch_handler;
pub mod blob_batch_sub_request;
pub mod blob_batch_sub_response;
pub mod blob_handler;
pub mod block_blob_handler;
pub mod container_handler;
pub mod i_page_blob_ranges_manager;
pub mod page_blob_handler;
pub mod page_blob_ranges_manager;
pub mod service_handler;
pub mod sub_response_text_body_stream;

#[derive(Debug, Clone, Default)]
pub struct BlobHandlersModule;
