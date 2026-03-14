use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use azurite_common::i_logger::ILogger;

use crate::errors::StorageError;
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedHttpRequest;
use crate::persistence::{container_public_access, IBlobMetadataStore};

use super::i_authenticator::IAuthenticator;

static CONTAINER_PUBLIC_READ_OPERATIONS: LazyLock<HashSet<Operation>> = LazyLock::new(|| {
    HashSet::from([
        Operation::Container_GetProperties,
        Operation::Container_GetPropertiesWithHead,
        Operation::Container_GetAccessPolicy,
        Operation::Container_ListBlobFlatSegment,
        Operation::Container_ListBlobHierarchySegment,
        Operation::Blob_Download,
        Operation::Blob_GetProperties,
        Operation::PageBlob_GetPageRanges,
        Operation::PageBlob_GetPageRangesDiff,
        Operation::BlockBlob_GetBlockList,
    ])
});

static BLOB_PUBLIC_READ_OPERATIONS: LazyLock<HashSet<Operation>> = LazyLock::new(|| {
    HashSet::from([
        Operation::Blob_Download,
        Operation::Blob_GetProperties,
        Operation::PageBlob_GetPageRanges,
        Operation::PageBlob_GetPageRangesDiff,
        Operation::BlockBlob_GetBlockList,
    ])
});

pub struct PublicAccessAuthenticator {
    blobMetadataStore: Arc<dyn IBlobMetadataStore + Send + Sync>,
    logger: Arc<dyn ILogger + Send + Sync>,
}

impl PublicAccessAuthenticator {
    pub fn new(
        blobMetadataStore: Arc<dyn IBlobMetadataStore + Send + Sync>,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            blobMetadataStore,
            logger,
        }
    }

    async fn getContainerPublicAccessType(
        &self,
        account: &str,
        container: &str,
        context: &Context,
    ) -> Option<String> {
        match self
            .blobMetadataStore
            .getContainerACL(context, account, container, None)
            .await
        {
            Ok(Some(containerModel)) => container_public_access(&containerModel.properties),
            Ok(None) => None,
            Err(_) => None,
        }
    }
}

#[async_trait]
impl IAuthenticator for PublicAccessAuthenticator {
    async fn validate(
        &self,
        _req: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<Option<bool>, StorageError> {
        self.logger.info(
            "PublicAccessAuthenticator:validate() Start validation against public access.",
            context.contextId().as_deref(),
        );

        let account = context
            .extras()
            .get("account")
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        let containerName = context
            .extras()
            .get("container")
            .and_then(|value| value.as_string());
        let _blobName = context
            .extras()
            .get("blob")
            .and_then(|value| value.as_string());

        if containerName.is_none() {
            return Ok(None);
        }
        let containerName = containerName.unwrap_or_default();

        let containerPublicAccessType = self
            .getContainerPublicAccessType(&account, &containerName, context)
            .await;
        if containerPublicAccessType.is_none() {
            return Ok(None);
        }
        let containerPublicAccessType = containerPublicAccessType.unwrap_or_default();

        let operation = context.operation().unwrap_or_else(|| {
            panic!(
                "PublicAccessAuthenticator:validate() Operation shouldn't be undefined. Please make sure DispatchMiddleware is hooked before authentication related middleware."
            )
        });

        if containerPublicAccessType == "container" {
            if CONTAINER_PUBLIC_READ_OPERATIONS.contains(&operation) {
                return Ok(Some(true));
            }
        } else if containerPublicAccessType == "blob" {
            if BLOB_PUBLIC_READ_OPERATIONS.contains(&operation) {
                return Ok(Some(true));
            }
        } else {
            panic!(
                "PublicAccessAuthenticator:validate() Unsupported containerPublicAccessType {containerPublicAccessType}"
            );
        }

        Ok(None)
    }
}
