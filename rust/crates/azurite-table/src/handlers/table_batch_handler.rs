use std::collections::HashSet;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    batch::{
        parse_batch_request, serialize_batch_response, serialize_general_error_part,
        serialize_response_part, serialize_storage_error_part, BatchRequestPart,
        MAX_BATCH_OPERATIONS, NO_PARTITION_KEY_ERROR, TOO_MANY_OPERATIONS_ERROR,
    },
    context::TableStorageContext,
    errors::{StorageError, StorageErrorFactory},
    generated::{
        artifacts::models::{GeneratedObject, GeneratedValue},
        context::Context,
        handlers::i_table_handler::ITableHandler,
        i_request::{GeneratedHttpRequest, HttpMethod, IRequest},
        i_response::GeneratedHttpResponse,
        utils::i_logger::ILogger,
    },
    middlewares::internalTableStorageContextMiddleware,
    utils::utils::extract_entity_keys_from_url_or_body,
};

use super::table_handler::TableHandler;

#[derive(Default)]
struct NoopLogger;

impl ILogger for NoopLogger {
    fn error(&self, _message: &str, _contextID: Option<&str>) {}
    fn warn(&self, _message: &str, _contextID: Option<&str>) {}
    fn info(&self, _message: &str, _contextID: Option<&str>) {}
    fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
    fn debug(&self, _message: &str, _contextID: Option<&str>) {}
}

#[derive(Clone)]
pub struct TableBatchHandler {
    context: Context,
    table_handler: TableHandler,
}

impl TableBatchHandler {
    pub fn new(context: &Context, table_handler: TableHandler) -> Self {
        Self {
            context: context.clone(),
            table_handler,
        }
    }

    pub async fn process_batch_request_and_serialize_response(
        &self,
        request_body: &str,
    ) -> Result<String, StorageError> {
        let envelope = parse_batch_request(request_body)
            .map_err(|_| StorageErrorFactory::getInvalidInput(&self.context, None))?;

        if envelope.requests.is_empty() {
            return Ok(serialize_batch_response(
                &envelope,
                &[serialize_general_error_part(
                    "The batch request contains no operations.",
                    self.context.contextId().as_deref(),
                )],
            ));
        }

        if envelope.requests.len() > MAX_BATCH_OPERATIONS {
            return Ok(serialize_batch_response(
                &envelope,
                &[serialize_general_error_part(
                    TOO_MANY_OPERATIONS_ERROR,
                    self.context.contextId().as_deref(),
                )],
            ));
        }

        let batch_id = Uuid::new_v4().to_string();
        self.table_handler
            .base
            .metadataStore
            .beginBatchTransaction(&batch_id)
            .await?;

        let mut response_parts = Vec::with_capacity(envelope.requests.len());
        let mut error_part = None;
        let mut batch_account = String::new();
        let mut batch_table = String::new();
        let mut batch_partition_key: Option<String> = None;
        let mut seen_rows = HashSet::new();

        for (index, request) in envelope.requests.iter().enumerate() {
            let content_id = index + 1;
            let (subcontext, table_context) = match self.prepare_subcontext(request, &batch_id) {
                Ok(result) => result,
                Err(error) => {
                    error_part = Some(serialize_storage_error_part(content_id, &error));
                    break;
                }
            };

            if batch_account.is_empty() {
                batch_account = table_context.account().unwrap_or_default();
                batch_table = table_context.tableName().unwrap_or_default();
            }

            if let Err(error) = self.validate_batch_request(
                request,
                &table_context,
                &mut batch_partition_key,
                &mut seen_rows,
            ) {
                error_part = Some(serialize_storage_error_part(content_id, &error));
                break;
            }

            match self.dispatch_request(request, subcontext).await {
                Ok(response) => response_parts.push(serialize_response_part(request, &response)),
                Err(error) => {
                    error_part = Some(serialize_storage_error_part(content_id, &error));
                    break;
                }
            }
        }

        let batch_success = error_part.is_none();
        self.table_handler
            .base
            .metadataStore
            .endBatchTransaction(
                &batch_account,
                &batch_table,
                &batch_id,
                &self.context,
                batch_success,
            )
            .await?;

        if let Some(error_part) = error_part {
            return Ok(serialize_batch_response(&envelope, &[error_part]));
        }

        Ok(serialize_batch_response(&envelope, &response_parts))
    }

    fn prepare_subcontext(
        &self,
        request: &BatchRequestPart,
        batch_id: &str,
    ) -> Result<(Context, TableStorageContext), StorageError> {
        let context = Context::with_path(self.context.path.clone());
        context.setRequest(Some(request.request.clone()));
        context.setResponse(Some(GeneratedHttpResponse::default()));
        context.setContextId(self.context.contextId());
        context.setStartTime(self.context.startTime().or_else(|| Some(Utc::now())));

        let mut response = GeneratedHttpResponse::default();
        let request_host = extract_request_host(&request.request);
        let request_path = request.request.getPath();
        internalTableStorageContextMiddleware(
            &context,
            &request.request,
            &mut response,
            &request_host,
            &request_path,
            &NoopLogger,
            true,
            false,
        )?;

        let table_context = TableStorageContext::new(&context);
        table_context.setBatchId(Some(batch_id.to_string()));

        let (partition_key, row_key) = extract_entity_keys_from_url_or_body(
            &request.request.getUrl(),
            request.request.getBody().as_deref(),
        );
        if table_context.partitionKey().is_none() {
            table_context.setPartitionKey(partition_key);
        }
        if table_context.rowKey().is_none() {
            table_context.setRowKey(row_key);
        }

        Ok((context, table_context))
    }

    fn validate_batch_request(
        &self,
        request: &BatchRequestPart,
        table_context: &TableStorageContext,
        batch_partition_key: &mut Option<String>,
        seen_rows: &mut HashSet<String>,
    ) -> Result<(), StorageError> {
        let method = request.request.getMethod();
        let partition_key = table_context.partitionKey();
        let row_key = table_context.rowKey();

        if batch_partition_key.is_none() {
            *batch_partition_key = partition_key.clone();
        }

        if matches!(method, HttpMethod::GET) {
            if batch_partition_key.is_none() {
                return Err(StorageErrorFactory::getInvalidInput(table_context, None));
            }
            if partition_key.as_deref() != batch_partition_key.as_deref() {
                return Err(StorageErrorFactory::getInvalidInput(table_context, None));
            }
            if row_key.is_none() {
                return Err(StorageErrorFactory::getNotImplementedError(table_context));
            }
            return Ok(());
        }

        let partition_key = partition_key.ok_or_else(|| {
            StorageErrorFactory::getInvalidInput(
                table_context,
                Some(std::collections::BTreeMap::from([(
                    String::from("MessageDetails"),
                    String::from(NO_PARTITION_KEY_ERROR),
                )])),
            )
        })?;

        if batch_partition_key.as_deref() != Some(partition_key.as_str()) {
            return Err(StorageErrorFactory::getInvalidInput(table_context, None));
        }

        let row_key =
            row_key.ok_or_else(|| StorageErrorFactory::getInvalidInput(table_context, None))?;
        let row_identifier = format!("{partition_key}\u{0}{row_key}");
        if !seen_rows.insert(row_identifier) {
            return Err(StorageErrorFactory::getBatchDuplicateRowKey(
                table_context,
                &row_key,
            ));
        }

        Ok(())
    }

    async fn dispatch_request(
        &self,
        request: &BatchRequestPart,
        context: Context,
    ) -> Result<crate::generated::artifacts::models::GeneratedResponse, StorageError> {
        let options = self.build_options(&request.request);
        match request.request.getMethod() {
            HttpMethod::POST => {
                self.table_handler
                    .insertEntity(
                        self.parse_entity(&request.request, &context)?,
                        options,
                        context,
                    )
                    .await
            }
            HttpMethod::PUT => {
                self.table_handler
                    .updateEntity(
                        self.parse_entity(&request.request, &context)?,
                        options,
                        context,
                    )
                    .await
            }
            HttpMethod::PATCH | HttpMethod::MERGE => {
                self.table_handler
                    .mergeEntity(
                        self.parse_entity(&request.request, &context)?,
                        options,
                        context,
                    )
                    .await
            }
            HttpMethod::DELETE => self.table_handler.deleteEntity(options, context).await,
            HttpMethod::GET => {
                self.table_handler
                    .queryEntitiesWithPartitionAndRowKey(options, context)
                    .await
            }
            _ => Err(StorageErrorFactory::getNotImplementedError(&context)),
        }
    }

    fn parse_entity(
        &self,
        request: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<GeneratedObject, StorageError> {
        let Some(body) = request.getBody() else {
            return Ok(GeneratedObject::new());
        };
        if body.trim().is_empty() {
            return Ok(GeneratedObject::new());
        }

        let value = serde_json::from_str::<serde_json::Value>(&body)
            .map_err(|_| StorageErrorFactory::getInvalidInput(context, None))?;
        match GeneratedValue::from(value) {
            GeneratedValue::Object(value) => Ok(value),
            _ => Err(StorageErrorFactory::getInvalidInput(context, None)),
        }
    }

    fn build_options(&self, request: &GeneratedHttpRequest) -> GeneratedObject {
        let mut options = GeneratedObject::new();

        if let Some(if_match) = request.getHeader("if-match") {
            options.insert(String::from("ifMatch"), GeneratedValue::String(if_match));
        }
        if let Some(request_id) = request.getHeader("x-ms-client-request-id") {
            options.insert(
                String::from("requestId"),
                GeneratedValue::String(request_id),
            );
        }

        let mut query_options = GeneratedObject::new();
        if let Some(format) = request.getQuery("$format") {
            query_options.insert(String::from("format"), GeneratedValue::String(format));
        }
        if let Some(select) = request.getQuery("$select") {
            query_options.insert(String::from("select"), GeneratedValue::String(select));
        }
        if let Some(filter) = request.getQuery("$filter") {
            query_options.insert(String::from("filter"), GeneratedValue::String(filter));
        }
        if let Some(top) = request
            .getQuery("$top")
            .and_then(|value| value.parse::<f64>().ok())
        {
            query_options.insert(String::from("top"), GeneratedValue::Number(top));
        }
        if !query_options.is_empty() {
            options.insert(
                String::from("queryOptions"),
                GeneratedValue::Object(query_options),
            );
        }

        options
    }
}

fn extract_request_host(request: &GeneratedHttpRequest) -> String {
    let endpoint = request.getEndpoint();
    endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(endpoint.as_str())
        .split('/')
        .next()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        generated::i_request::{GeneratedHttpRequest, HttpMethod, RequestHeaderValue},
        handlers::BaseHandler,
        persistence::{ITableMetadataStore, LokiTableMetadataStore, Table},
    };

    use super::*;

    fn batch_context() -> Context {
        let mut request = GeneratedHttpRequest::with_details(
            HttpMethod::POST,
            "http://127.0.0.1:10002/devstoreaccount1/$batch",
            "http://127.0.0.1:10002",
            "/devstoreaccount1/$batch",
        );
        request.protocol = String::from("http");
        request.headers.insert(
            String::from("content-type"),
            RequestHeaderValue::Single(String::from("multipart/mixed; boundary=batch_test")),
        );

        let context = Context::from_holder(
            Context::new_holder(),
            "azurite_table_context",
            Some(request),
            Some(GeneratedHttpResponse::default()),
        );
        context.setContextId(Some(String::from("req-1")));
        context.setStartTime(Some(Utc::now()));
        context
    }

    async fn test_handler() -> (Arc<LokiTableMetadataStore>, TableBatchHandler, Context) {
        let store = Arc::new(LokiTableMetadataStore::new(
            "table-batch-handler-tests",
            true,
        ));
        store.init().await.unwrap();
        store
            .createTable(
                &Context::default(),
                Table {
                    account: String::from("devstoreaccount1"),
                    table: String::from("Customers"),
                    ..Table::default()
                },
            )
            .await
            .unwrap();

        let handler = TableHandler::new(BaseHandler::new(store.clone(), Arc::new(NoopLogger)));
        let context = batch_context();
        let batch_handler = TableBatchHandler::new(&context, handler);
        (store, batch_handler, context)
    }

    #[tokio::test]
    async fn processes_insert_batch_requests() {
        let (store, batch_handler, context) = test_handler().await;
        let body = "--batch_test\r\nContent-Type: multipart/mixed; boundary=changeset_test\r\n\r\n--changeset_test\r\nContent-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\nPOST http://127.0.0.1:10002/devstoreaccount1/Customers HTTP/1.1\r\nAccept: application/json;odata=nometadata\r\nContent-Type: application/json\r\n\r\n{\"PartitionKey\":\"pk\",\"RowKey\":\"rk\",\"Name\":\"Ada\"}\r\n--changeset_test--\r\n--batch_test--\r\n";

        let response = batch_handler
            .process_batch_request_and_serialize_response(body)
            .await
            .unwrap();

        assert!(response.contains("HTTP/1.1 201 Created"));
        let entity = store
            .queryTableEntitiesWithPartitionAndRowKey(
                &context,
                "Customers",
                "devstoreaccount1",
                "pk",
                "rk",
                None,
            )
            .await
            .unwrap();
        assert!(entity.is_some());
    }

    #[tokio::test]
    async fn rolls_back_when_batch_fails() {
        let (store, batch_handler, context) = test_handler().await;
        let body = "--batch_test\r\nContent-Type: multipart/mixed; boundary=changeset_test\r\n\r\n--changeset_test\r\nContent-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\nPOST http://127.0.0.1:10002/devstoreaccount1/Customers HTTP/1.1\r\nAccept: application/json;odata=nometadata\r\nContent-Type: application/json\r\n\r\n{\"PartitionKey\":\"pk\",\"RowKey\":\"rk\"}\r\n--changeset_test\r\nContent-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\nPOST http://127.0.0.1:10002/devstoreaccount1/Customers HTTP/1.1\r\nAccept: application/json;odata=nometadata\r\nContent-Type: application/json\r\n\r\n{\"PartitionKey\":\"pk\",\"RowKey\":\"rk\"}\r\n--changeset_test--\r\n--batch_test--\r\n";

        let response = batch_handler
            .process_batch_request_and_serialize_response(body)
            .await
            .unwrap();

        assert!(response.contains("HTTP/1.1 400 Bad Request"));
        let entity = store
            .queryTableEntitiesWithPartitionAndRowKey(
                &context,
                "Customers",
                "devstoreaccount1",
                "pk",
                "rk",
                None,
            )
            .await
            .unwrap();
        assert!(entity.is_none());
    }
}
