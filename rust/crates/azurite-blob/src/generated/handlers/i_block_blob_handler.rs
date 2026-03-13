use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedReadableStream;

#[allow(non_snake_case)]
#[async_trait]
pub trait IBlockBlobHandler: Send + Sync {
    async fn upload(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::BlockBlobUploadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobUploadResponse>;
    async fn putBlobFromUrl(
        &self,
        contentLength: f64,
        copySource: String,
        options: models::BlockBlobPutBlobFromUrlOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobPutBlobFromUrlResponse>;
    async fn stageBlock(
        &self,
        blockId: String,
        contentLength: f64,
        body: GeneratedReadableStream,
        options: models::BlockBlobStageBlockOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobStageBlockResponse>;
    async fn stageBlockFromURL(
        &self,
        blockId: String,
        contentLength: f64,
        sourceUrl: String,
        options: models::BlockBlobStageBlockFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobStageBlockFromURLResponse>;
    async fn commitBlockList(
        &self,
        blocks: models::BlockLookupList,
        options: models::BlockBlobCommitBlockListOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobCommitBlockListResponse>;
    async fn getBlockList(
        &self,
        options: models::BlockBlobGetBlockListOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobGetBlockListResponse>;
}
