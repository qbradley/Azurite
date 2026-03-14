use async_trait::async_trait;

use crate::errors::StorageError;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedHttpRequest;

#[async_trait]
pub trait IAuthenticator: Send + Sync {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<Option<bool>, StorageError>;
}
