use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::context::queue_storage_context::QueueStorageContext;
use crate::errors::StorageErrorFactory;
use crate::generated::artifacts::models::{
    GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue, QueueCreateOptionalParams,
    QueueCreateResponse, QueueDeleteMethodOptionalParams, QueueDeleteResponse,
    QueueGetAccessPolicyOptionalParams, QueueGetAccessPolicyResponse,
    QueueGetAccessPolicyWithHeadOptionalParams, QueueGetAccessPolicyWithHeadResponse,
    QueueGetPropertiesOptionalParams, QueueGetPropertiesResponse,
    QueueGetPropertiesWithHeadOptionalParams, QueueGetPropertiesWithHeadResponse,
    QueueSetAccessPolicyOptionalParams, QueueSetAccessPolicyResponse,
    QueueSetMetadataOptionalParams, QueueSetMetadataResponse,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_queue_handler::IQueueHandler;
use crate::generated::i_request::IRequest;
use crate::persistence::QueueModel;
use crate::utils::constants::QUEUE_SERVICE_PERMISSION;

use super::base_handler::{get_optional_object_array, metadata_to_object, BaseHandler};

#[derive(Clone)]
pub struct QueueHandler {
    pub base: BaseHandler,
}

impl QueueHandler {
    pub fn new(base: BaseHandler) -> Self {
        Self { base }
    }

    fn parse_metadata(
        req_metadata: Option<&GeneratedObject>,
        headers: &[String],
    ) -> Option<BTreeMap<String, String>> {
        let req_metadata = req_metadata?;
        let meta_prefix = crate::utils::constants::HeaderConstants.X_MS_META;
        let mut metadata = BTreeMap::new();

        for (item, value) in req_metadata {
            let Some(value) = value.as_string() else {
                continue;
            };

            for header in headers {
                if format!("{meta_prefix}{item}") == header.to_ascii_lowercase() {
                    metadata.insert(header[meta_prefix.len()..].to_string(), value.clone());
                    break;
                }
            }
        }

        if metadata.is_empty() {
            None
        } else {
            Some(metadata)
        }
    }
}

#[async_trait]
impl IQueueHandler for QueueHandler {
    async fn create(
        &self,
        options: QueueCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueCreateResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();
        let raw_headers = context
            .request()
            .map(|request| request.getRawHeaders())
            .unwrap_or_default();
        let metadata = Self::parse_metadata(
            options.get("metadata").and_then(GeneratedValue::as_object),
            &raw_headers,
        );

        let status_code = self
            .base
            .metadataStore
            .createQueue(
                QueueModel {
                    accountName: account_name,
                    name: queue_name,
                    metadata,
                    queueAcl: None,
                    properties: GeneratedObject::new(),
                },
                Some(&context),
            )
            .await?;

        let mut response = GeneratedResponse::new(status_code as u16);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn delete(
        &self,
        options: QueueDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueDeleteResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();

        self.base
            .metadataStore
            .deleteQueue(&account_name, &queue_name, Some(&context))
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn getProperties(
        &self,
        options: QueueGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueGetPropertiesResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();

        let queue = self
            .base
            .metadataStore
            .getQueue(&account_name, &queue_name, Some(&context))
            .await?;
        let approximate_messages_count = self
            .base
            .metadataStore
            .getMessagesCount(&queue.accountName, &queue.name, Some(&context))
            .await?;

        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "approximateMessagesCount",
            GeneratedValue::Number(approximate_messages_count as f64),
        );
        if let Some(metadata) = &queue.metadata {
            response.insert_field(
                "metadata",
                GeneratedValue::Object(metadata_to_object(metadata)),
            );
        }
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn getPropertiesWithHead(
        &self,
        options: QueueGetPropertiesWithHeadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueGetPropertiesWithHeadResponse> {
        self.getProperties(options, context).await
    }

    async fn setMetadata(
        &self,
        options: QueueSetMetadataOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueSetMetadataResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();
        let raw_headers = context
            .request()
            .map(|request| request.getRawHeaders())
            .unwrap_or_default();
        let metadata = Self::parse_metadata(
            options.get("metadata").and_then(GeneratedValue::as_object),
            &raw_headers,
        );

        self.base
            .metadataStore
            .setQueueMetadata(&account_name, &queue_name, metadata, Some(&context))
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn getAccessPolicy(
        &self,
        options: QueueGetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueGetAccessPolicyResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();

        let queue = self
            .base
            .metadataStore
            .getQueue(&account_name, &queue_name, Some(&context))
            .await?;

        let body = queue
            .queueAcl
            .unwrap_or_default()
            .into_iter()
            .map(GeneratedValue::Object)
            .collect();
        let mut response = GeneratedResponse::new(200);
        response.body = Some(GeneratedBody::Value(GeneratedValue::Array(body)));
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn getAccessPolicyWithHead(
        &self,
        options: QueueGetAccessPolicyWithHeadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueGetAccessPolicyWithHeadResponse> {
        self.getAccessPolicy(options, context).await
    }

    async fn setAccessPolicy(
        &self,
        options: QueueSetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<QueueSetAccessPolicyResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();

        let queue_acl = get_optional_object_array(&options, "queueAcl").map(|values| {
            values
                .into_iter()
                .map(normalize_signed_identifier)
                .collect::<Vec<_>>()
        });

        if let Some(queue_acl) = &queue_acl {
            if queue_acl.len() > 5 {
                return Err(Box::new(StorageErrorFactory::getInvalidXmlDocument(
                    context.contextId().as_deref(),
                    None,
                )));
            }

            for acl in queue_acl {
                if let Some(permission) = acl
                    .get("accessPolicy")
                    .and_then(GeneratedValue::as_object)
                    .and_then(|access_policy| access_policy.get("permission"))
                    .and_then(GeneratedValue::as_string)
                {
                    if permission
                        .chars()
                        .any(|permission| !QUEUE_SERVICE_PERMISSION.contains(permission))
                    {
                        return Err(Box::new(StorageErrorFactory::getInvalidXmlDocument(
                            context.contextId().as_deref(),
                            None,
                        )));
                    }
                }
            }
        }

        self.base
            .metadataStore
            .setQueueACL(&account_name, &queue_name, queue_acl, Some(&context))
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }
}

fn normalize_signed_identifier(mut identifier: GeneratedObject) -> GeneratedObject {
    rename_key(&mut identifier, "Id", "id");
    rename_key(&mut identifier, "AccessPolicy", "accessPolicy");
    if let Some(GeneratedValue::Object(access_policy)) = identifier.get_mut("accessPolicy") {
        rename_key(access_policy, "Start", "start");
        rename_key(access_policy, "Expiry", "expiry");
        rename_key(access_policy, "Permission", "permission");
    }
    identifier
}

fn rename_key(object: &mut GeneratedObject, from: &str, to: &str) {
    if object.contains_key(to) {
        return;
    }
    if let Some(value) = object.remove(from) {
        object.insert(to.to_string(), value);
    }
}
