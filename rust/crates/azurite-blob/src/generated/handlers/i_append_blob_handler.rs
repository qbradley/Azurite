use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedReadableStream;

#[allow(non_snake_case)]
#[async_trait]
pub trait IAppendBlobHandler: Send + Sync {
    async fn create(
        &self,
        contentLength: f64,
        options: models::AppendBlobCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobCreateResponse>;
    async fn appendBlock(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::AppendBlobAppendBlockOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobAppendBlockResponse>;
    async fn appendBlockFromUrl(
        &self,
        sourceUrl: String,
        contentLength: f64,
        options: models::AppendBlobAppendBlockFromUrlOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobAppendBlockFromUrlResponse>;
    async fn seal(
        &self,
        options: models::AppendBlobSealOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobSealResponse>;
}
