use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedReadableStream;

#[allow(non_snake_case)]
#[async_trait]
pub trait IPageBlobHandler: Send + Sync {
    async fn create(
        &self,
        contentLength: f64,
        blobContentLength: f64,
        options: models::PageBlobCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobCreateResponse>;
    async fn uploadPages(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::PageBlobUploadPagesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobUploadPagesResponse>;
    async fn clearPages(
        &self,
        contentLength: f64,
        options: models::PageBlobClearPagesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobClearPagesResponse>;
    async fn uploadPagesFromURL(
        &self,
        sourceUrl: String,
        sourceRange: String,
        contentLength: f64,
        range: String,
        options: models::PageBlobUploadPagesFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobUploadPagesFromURLResponse>;
    async fn getPageRanges(
        &self,
        options: models::PageBlobGetPageRangesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobGetPageRangesResponse>;
    async fn getPageRangesDiff(
        &self,
        options: models::PageBlobGetPageRangesDiffOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobGetPageRangesDiffResponse>;
    async fn resize(
        &self,
        blobContentLength: f64,
        options: models::PageBlobResizeOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobResizeResponse>;
    async fn updateSequenceNumber(
        &self,
        sequenceNumberAction: models::SequenceNumberActionType,
        options: models::PageBlobUpdateSequenceNumberOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobUpdateSequenceNumberResponse>;
    async fn copyIncremental(
        &self,
        copySource: String,
        options: models::PageBlobCopyIncrementalOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobCopyIncrementalResponse>;
}
