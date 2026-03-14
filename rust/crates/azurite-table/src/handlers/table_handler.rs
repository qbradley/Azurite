use async_trait::async_trait;
use base64::prelude::*;

use crate::context::TableStorageContext;
use crate::entity::{to_annotation_level, NormalizedEntity};
use crate::errors::StorageErrorFactory;
use crate::generated::artifacts::models::{
    GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue, SignedIdentifier,
    TableBatchOptionalParams, TableBatchResponse, TableCreateOptionalParams, TableCreateResponse,
    TableDeleteEntityOptionalParams, TableDeleteEntityResponse, TableDeleteMethodOptionalParams,
    TableDeleteResponse, TableEntity, TableGetAccessPolicyOptionalParams,
    TableGetAccessPolicyResponse, TableInsertEntityOptionalParams, TableInsertEntityResponse,
    TableMergeEntityOptionalParams, TableMergeEntityResponse, TableProperties,
    TableQueryEntitiesOptionalParams, TableQueryEntitiesResponse,
    TableQueryEntitiesWithPartitionAndRowKeyOptionalParams,
    TableQueryEntitiesWithPartitionAndRowKeyResponse, TableQueryOptionalParams, TableQueryResponse,
    TableSetAccessPolicyOptionalParams, TableSetAccessPolicyResponse,
    TableUpdateEntityOptionalParams, TableUpdateEntityResponse,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_table_handler::ITableHandler;
use crate::generated::i_request::IRequest;
use crate::persistence::{
    AccessPolicy, Entity, SignedIdentifier as PersistedSignedIdentifier, Table,
};
use crate::utils::constants::{
    BODY_SIZE_MAX, DEFAULT_KEY_MAX_LENGTH, ENTITY_SIZE_MAX, FULL_METADATA_ACCEPT,
    MINIMAL_METADATA_ACCEPT, ODATA_TYPE, TABLE_SERVICE_PERMISSION,
};
use crate::utils::utils::{
    getEntityOdataAnnotationsForResponse, getTableOdataAnnotationsForResponse,
    getTablePropertiesOdataAnnotationsForResponse, getUTF8ByteSize, isEtagValid,
    newHighPrecisionTimeStamp, newTableEntityEtag, updateTableOptionalOdataAnnotationsForResponse,
    validateTableName, ODataAnnotationsOptional,
};

use super::base_handler::{
    add_optional_string_field, generated_object_to_json_map, get_query_options, get_string,
    parse_select_set, string_value, BaseHandler,
};
use super::table_batch_handler::TableBatchHandler;

#[derive(Clone)]
pub struct TableHandler {
    pub base: BaseHandler,
}

impl TableHandler {
    pub fn new(base: BaseHandler) -> Self {
        Self { base }
    }

    fn account_name(
        &self,
        context: &TableStorageContext,
    ) -> Result<String, crate::errors::StorageError> {
        context
            .account()
            .ok_or_else(|| StorageErrorFactory::getAccountNameEmpty(context))
    }

    fn table_name(
        &self,
        context: &TableStorageContext,
    ) -> Result<String, crate::errors::StorageError> {
        context
            .tableName()
            .ok_or_else(|| StorageErrorFactory::getTableNameEmpty(context))
    }

    fn get_required_entity_keys(
        &self,
        context: &TableStorageContext,
        entity: Option<&GeneratedObject>,
    ) -> Result<(String, String), crate::errors::StorageError> {
        let partition_key = entity
            .and_then(|entity| get_string(entity, "PartitionKey"))
            .or_else(|| context.partitionKey())
            .ok_or_else(|| StorageErrorFactory::getPropertiesNeedValue(context))?;
        let row_key = entity
            .and_then(|entity| get_string(entity, "RowKey"))
            .or_else(|| context.rowKey())
            .ok_or_else(|| StorageErrorFactory::getPropertiesNeedValue(context))?;
        Ok((partition_key, row_key))
    }

    fn create_persisted_entity(
        &self,
        context: &Context,
        properties: Option<GeneratedObject>,
        partition_key: String,
        row_key: String,
    ) -> Entity {
        let mod_time = newHighPrecisionTimeStamp(BaseHandler::start_time(context));
        let e_tag = newTableEntityEtag(&mod_time);
        Entity {
            PartitionKey: partition_key,
            RowKey: row_key,
            properties: properties.unwrap_or_default(),
            lastModifiedTime: mod_time,
            eTag: e_tag,
            ..Entity::default()
        }
    }

    fn validate_key(
        &self,
        context: &Context,
        key: &str,
    ) -> Result<(), crate::errors::StorageError> {
        if key.len() > DEFAULT_KEY_MAX_LENGTH {
            return Err(StorageErrorFactory::getInvalidInput(context, None));
        }
        if key
            .chars()
            .any(|ch| matches!(ch, '\u{0000}'..='\u{001f}' | '\u{007f}'..='\u{009f}' | '/' | '\\' | '#' | '?'))
        {
            return Err(StorageErrorFactory::getInvalidInput(context, None));
        }
        Ok(())
    }

    fn check_body_limit(
        &self,
        context: &Context,
        body: Option<&str>,
    ) -> Result<(), crate::errors::StorageError> {
        if let Some(body) = body {
            if getUTF8ByteSize(body) > BODY_SIZE_MAX {
                return Err(StorageErrorFactory::getRequestBodyTooLarge(context));
            }
        }
        Ok(())
    }

    fn check_entity_limit(
        &self,
        context: &Context,
        body: Option<&str>,
    ) -> Result<(), crate::errors::StorageError> {
        if let Some(body) = body {
            if getUTF8ByteSize(body) > ENTITY_SIZE_MAX {
                return Err(StorageErrorFactory::getEntityTooLarge(context));
            }
        }
        Ok(())
    }

    fn check_properties(
        &self,
        context: &Context,
        properties: &GeneratedObject,
    ) -> Result<(), crate::errors::StorageError> {
        let property_count = properties
            .keys()
            .filter(|key| !key.ends_with(ODATA_TYPE))
            .count();
        if property_count > 255 {
            return Err(StorageErrorFactory::getInvalidInput(context, None));
        }

        for (prop, value) in properties {
            if prop.ends_with(ODATA_TYPE) {
                continue;
            }
            let type_key = format!("{prop}{ODATA_TYPE}");
            let edm_type = properties
                .get(&type_key)
                .and_then(GeneratedValue::as_string);
            match value {
                GeneratedValue::Null => {}
                GeneratedValue::String(text) => {
                    if matches!(edm_type.as_deref(), Some("Edm.Binary")) {
                        if BASE64_STANDARD
                            .decode(text)
                            .map(|bytes| bytes.len())
                            .unwrap_or(usize::MAX)
                            > 64 * 1024
                        {
                            return Err(StorageErrorFactory::getPropertyValueTooLargeError(
                                context,
                            ));
                        }
                    } else if text.len() > 32 * 1024 {
                        return Err(StorageErrorFactory::getPropertyValueTooLargeError(context));
                    } else if text.is_empty() && matches!(edm_type.as_deref(), Some("Edm.DateTime"))
                    {
                        return Err(StorageErrorFactory::getInvalidInput(context, None));
                    }
                }
                GeneratedValue::Array(values) if values.len() > 32 * 1024 => {
                    return Err(StorageErrorFactory::getPropertyValueTooLargeError(context));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn remove_etag_property(&self, properties: &mut GeneratedObject) {
        properties.remove("odata.etag");
    }

    fn build_normalized_entity(
        &self,
        entity: &Entity,
        context: &Context,
    ) -> Result<NormalizedEntity, crate::errors::StorageError> {
        let mut properties = generated_object_to_json_map(&entity.properties);
        properties.insert(
            String::from("PartitionKey"),
            serde_json::Value::String(entity.PartitionKey.clone()),
        );
        properties.insert(
            String::from("RowKey"),
            serde_json::Value::String(entity.RowKey.clone()),
        );
        properties.insert(
            String::from("lastModifiedTime"),
            serde_json::Value::String(entity.lastModifiedTime.clone()),
        );
        NormalizedEntity::new(crate::entity::entity_property::Entity { properties })
            .map_err(|_| StorageErrorFactory::getInvalidInput(context, None))
    }

    fn validate_entity_shape(
        &self,
        context: &Context,
        entity: &Entity,
    ) -> Result<(), crate::errors::StorageError> {
        let normalized = self.build_normalized_entity(entity, context)?;
        let _ = normalized.normalize();
        Ok(())
    }

    fn build_entity_response_body(
        &self,
        entity: &Entity,
        accept: &str,
        injections: GeneratedObject,
        includes: Option<&std::collections::HashSet<String>>,
        context: &Context,
    ) -> Result<String, crate::errors::StorageError> {
        let normalized = self.build_normalized_entity(entity, context)?;
        let annotation_level = to_annotation_level(accept)
            .map_err(|_| StorageErrorFactory::getAtomFormatNotSupported(context))?;
        let injections = injections
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    serde_json::to_string(&value.to_json_value())
                        .unwrap_or_else(|_| String::from("null")),
                )
            })
            .collect();
        Ok(normalized.to_response_string(annotation_level, injections, includes))
    }

    fn entity_injection_fields(
        &self,
        account: &str,
        table: &str,
        partition_key: &str,
        row_key: &str,
        accept: &str,
        e_tag: &str,
        context: &Context,
    ) -> GeneratedObject {
        let annotations = getEntityOdataAnnotationsForResponse(
            account,
            table,
            &self.base.odata_annotation_url_prefix(context, account),
            Some(partition_key),
            Some(row_key),
            Some(accept),
        );
        let mut object = GeneratedObject::new();
        if accept == MINIMAL_METADATA_ACCEPT || accept == FULL_METADATA_ACCEPT {
            if let Some(metadata) = annotations.odatametadata.as_ref() {
                object.insert(
                    String::from("odata.metadata"),
                    string_value(metadata.clone()),
                );
            }
            object.insert(String::from("odata.etag"), string_value(e_tag.to_string()));
        }
        if accept == FULL_METADATA_ACCEPT {
            if let Some(odata_type) = annotations.odatatype {
                object.insert(String::from("odata.type"), string_value(odata_type));
            }
            if let Some(odata_id) = annotations.odataid {
                object.insert(String::from("odata.id"), string_value(odata_id));
            }
            if let Some(edit_link) = annotations.odataeditLink {
                object.insert(String::from("odata.editLink"), string_value(edit_link));
            }
        }
        object
    }

    fn query_entities_response_body(
        &self,
        account: &str,
        table: &str,
        entities: &[Entity],
        accept: &str,
        select_set: Option<&std::collections::HashSet<String>>,
        context: &Context,
    ) -> Result<String, crate::errors::StorageError> {
        let mut body = serde_json::Map::new();
        if let Some(metadata) = getEntityOdataAnnotationsForResponse(
            account,
            table,
            &self.base.odata_annotation_url_prefix(context, account),
            Some(""),
            Some(""),
            Some(accept),
        )
        .odatametadata
        {
            body.insert(
                String::from("odata.metadata"),
                serde_json::Value::String(metadata),
            );
        }

        let mut values = Vec::with_capacity(entities.len());
        for entity in entities {
            let injections = self.entity_injection_fields(
                account,
                table,
                &entity.PartitionKey,
                &entity.RowKey,
                accept,
                &entity.eTag,
                context,
            );
            let json =
                self.build_entity_response_body(entity, accept, injections, select_set, context)?;
            values.push(
                serde_json::from_str::<serde_json::Value>(&json).unwrap_or(serde_json::Value::Null),
            );
        }
        body.insert(String::from("value"), serde_json::Value::Array(values));
        serde_json::to_string(&serde_json::Value::Object(body))
            .map_err(|_| StorageErrorFactory::getInvalidInput(context, None))
    }

    fn check_update_if_match(
        &self,
        context: &Context,
        if_match: Option<&str>,
    ) -> Result<(), crate::errors::StorageError> {
        if if_match == Some("") {
            return Err(StorageErrorFactory::getPreconditionFailed(context));
        }
        if let Some(value) = if_match {
            if value != "*" && isEtagValid(value) {
                return Err(StorageErrorFactory::getInvalidInput(context, None));
            }
        }
        Ok(())
    }

    fn check_merge_if_match(
        &self,
        context: &Context,
        if_match: Option<&str>,
    ) -> Result<(), crate::errors::StorageError> {
        if let Some(value) = if_match {
            if value != "*" && !value.is_empty() && isEtagValid(value) {
                return Err(StorageErrorFactory::getInvalidOperation(context, None));
            }
        }
        Ok(())
    }

    fn table_response_value(
        &self,
        account: &str,
        table: &str,
        accept: &str,
        context: &Context,
    ) -> GeneratedObject {
        let annotations = updateTableOptionalOdataAnnotationsForResponse(
            ODataAnnotationsOptional::default(),
            account,
            table,
            &self.base.odata_annotation_url_prefix(context, account),
            Some(accept),
        );
        let mut response = GeneratedObject::new();
        response.insert(String::from("tableName"), string_value(table.to_string()));
        add_optional(&mut response, "odatametadata", annotations.odatametadata);
        add_optional(&mut response, "odatatype", annotations.odatatype);
        add_optional(&mut response, "odataid", annotations.odataid);
        add_optional(&mut response, "odataeditLink", annotations.odataeditLink);
        response
    }
}

#[async_trait]
impl ITableHandler for TableHandler {
    async fn query(
        &self,
        options: TableQueryOptionalParams,
        context: Context,
    ) -> Result<TableQueryResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let accept = self.base.get_and_check_payload_format(&context)?;
        let query_options = get_query_options(&options);
        let next_table_name = get_string(&options, "nextTableName");

        let (tables, continuation) = self
            .base
            .metadataStore
            .queryTable(
                &context,
                &account,
                query_options,
                next_table_name.as_deref(),
            )
            .await?;

        let mut response = GeneratedResponse::new(200);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        add_optional_string_field(&mut response, "xMsContinuationNextTableName", continuation);

        if accept == MINIMAL_METADATA_ACCEPT || accept == FULL_METADATA_ACCEPT {
            let annotation = getTableOdataAnnotationsForResponse(
                &account,
                "",
                &self.base.odata_annotation_url_prefix(&context, &account),
            );
            response.insert_field("odatametadata", string_value(annotation.odatametadata));
        }

        let table_values = tables
            .iter()
            .map(|item| {
                let props = getTablePropertiesOdataAnnotationsForResponse(
                    &item.table,
                    &account,
                    &self.base.odata_annotation_url_prefix(&context, &account),
                    Some(&accept),
                );
                let mut value = GeneratedObject::new();
                value.insert(String::from("tableName"), string_value(props.tableName));
                add_optional(&mut value, "odatatype", props.odatatype);
                add_optional(&mut value, "odataid", props.odataid);
                add_optional(&mut value, "odataeditLink", props.odataeditLink);
                GeneratedValue::Object(value)
            })
            .collect();
        response.insert_field("value", GeneratedValue::Array(table_values));
        self.base
            .set_response_content_type(&mut response, Some(&accept));
        Ok(response)
    }

    async fn create(
        &self,
        table: TableProperties,
        options: TableCreateOptionalParams,
        context: Context,
    ) -> Result<TableCreateResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let accept = self.base.get_and_check_payload_format(&context)?;
        let table_name = get_string(&table, "tableName")
            .or_else(|| get_string(&table, "TableName"))
            .ok_or_else(|| StorageErrorFactory::getTableNameEmpty(&context))?;
        validateTableName(&context, &table_name)?;

        self.base
            .metadataStore
            .createTable(
                &context,
                Table {
                    account: account.clone(),
                    table: table_name.clone(),
                    ..Table::default()
                },
            )
            .await?;

        let mut response = GeneratedResponse::new(201);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        response
            .fields
            .extend(self.table_response_value(&account, &table_name, &accept, &context));
        self.base.update_response_prefer(&mut response, &context);
        self.base
            .set_response_content_type(&mut response, Some(&accept));
        Ok(response)
    }

    async fn batch(
        &self,
        body: crate::generated::i_request::GeneratedReadableStream,
        options: TableBatchOptionalParams,
        context: Context,
    ) -> Result<TableBatchResponse, crate::errors::StorageError> {
        let request_body = body.read_to_string();
        self.check_body_limit(&context, Some(&request_body))?;
        let batch_handler = TableBatchHandler::new(&context);
        let response_body = batch_handler
            .process_batch_request_and_serialize_response(&request_body)
            .await?;

        let content_type_response = context
            .request()
            .and_then(|request| request.getHeader("content-type"))
            .map(|value| value.replace("batch", "batchresponse"));

        let mut response = GeneratedResponse::new(202);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        response.body = Some(BaseHandler::stream_body(response_body));
        response.contentType = content_type_response;
        Ok(response)
    }

    async fn delete(
        &self,
        options: TableDeleteMethodOptionalParams,
        context: Context,
    ) -> Result<TableDeleteResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        let accept = self.base.get_and_check_payload_format(&context)?;

        self.base
            .metadataStore
            .deleteTable(&context, &table, &account)
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        self.base
            .set_response_content_type(&mut response, Some(&accept));
        Ok(response)
    }

    async fn queryEntities(
        &self,
        options: TableQueryEntitiesOptionalParams,
        context: Context,
    ) -> Result<TableQueryEntitiesResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        let accept = self.base.get_and_check_payload_format(&context)?;
        self.check_body_limit(
            &context,
            context
                .request()
                .and_then(|request| request.getBody())
                .as_deref(),
        )?;

        let query_options = get_query_options(&options);
        let next_partition_key = get_string(&options, "nextPartitionKey");
        let next_row_key = get_string(&options, "nextRowKey");
        let (result, continuation_partition, continuation_row) = self
            .base
            .metadataStore
            .queryTableEntities(
                &context,
                &account,
                &table,
                query_options,
                next_partition_key.as_deref(),
                next_row_key.as_deref(),
            )
            .await?;

        let select_set = parse_select_set(&options);
        let body = self.query_entities_response_body(
            &account,
            &table,
            &result,
            &accept,
            select_set.as_ref(),
            &context,
        )?;

        let mut response = GeneratedResponse::new(200);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        add_optional_string_field(
            &mut response,
            "xMsContinuationNextPartitionKey",
            continuation_partition,
        );
        add_optional_string_field(&mut response, "xMsContinuationNextRowKey", continuation_row);
        response.body = Some(BaseHandler::stream_body(body));
        self.base
            .set_response_content_type(&mut response, Some(&accept));
        Ok(response)
    }

    async fn queryEntitiesWithPartitionAndRowKey(
        &self,
        options: TableQueryEntitiesWithPartitionAndRowKeyOptionalParams,
        context: Context,
    ) -> Result<TableQueryEntitiesWithPartitionAndRowKeyResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        let accept = self.base.get_and_check_payload_format(&context)?;
        let (partition_key, row_key) = self.get_required_entity_keys(&table_context, None)?;

        let entity = self
            .base
            .metadataStore
            .queryTableEntitiesWithPartitionAndRowKey(
                &context,
                &table,
                &account,
                &partition_key,
                &row_key,
                table_context.batchId().as_deref(),
            )
            .await?
            .ok_or_else(|| StorageErrorFactory::getEntityNotFound(&context))?;

        let select_set = parse_select_set(&options);
        let injections = self.entity_injection_fields(
            &account,
            &table,
            &partition_key,
            &row_key,
            &accept,
            &entity.eTag,
            &context,
        );
        let body = self.build_entity_response_body(
            &entity,
            &accept,
            injections,
            select_set.as_ref(),
            &context,
        )?;

        let mut response = GeneratedResponse::new(200);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        response.insert_field("eTag", string_value(entity.eTag));
        response.body = Some(BaseHandler::stream_body(body));
        self.base
            .set_response_content_type(&mut response, Some(&accept));
        Ok(response)
    }

    async fn updateEntity(
        &self,
        mut entity: TableEntity,
        options: TableUpdateEntityOptionalParams,
        context: Context,
    ) -> Result<TableUpdateEntityResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        self.check_entity_limit(
            &context,
            context
                .request()
                .and_then(|request| request.getBody())
                .as_deref(),
        )?;

        let (partition_key, row_key) =
            self.get_required_entity_keys(&table_context, Some(&entity))?;
        let if_match = get_string(&options, "ifMatch");
        self.check_update_if_match(&context, if_match.as_deref())?;
        self.validate_key(&context, &partition_key)?;
        self.validate_key(&context, &row_key)?;
        self.check_properties(&context, &entity)?;
        self.remove_etag_property(&mut entity);

        let persisted =
            self.create_persisted_entity(&context, Some(entity), partition_key, row_key);
        self.validate_entity_shape(&context, &persisted)?;
        let persisted = self
            .base
            .metadataStore
            .insertOrUpdateTableEntity(
                &context,
                &table,
                &account,
                persisted,
                if_match.as_deref(),
                table_context.batchId().as_deref(),
            )
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        response.insert_field("eTag", string_value(persisted.eTag));
        Ok(response)
    }

    async fn mergeEntity(
        &self,
        mut entity: TableEntity,
        options: TableMergeEntityOptionalParams,
        context: Context,
    ) -> Result<TableMergeEntityResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        self.check_entity_limit(
            &context,
            context
                .request()
                .and_then(|request| request.getBody())
                .as_deref(),
        )?;

        let (partition_key, row_key) =
            self.get_required_entity_keys(&table_context, Some(&entity))?;
        let if_match = get_string(&options, "ifMatch");
        self.check_merge_if_match(&context, if_match.as_deref())?;
        self.validate_key(&context, &partition_key)?;
        self.validate_key(&context, &row_key)?;
        if !entity.is_empty() {
            self.check_properties(&context, &entity)?;
        }
        self.remove_etag_property(&mut entity);

        let persisted =
            self.create_persisted_entity(&context, Some(entity), partition_key, row_key);
        self.validate_entity_shape(&context, &persisted)?;
        let persisted = self
            .base
            .metadataStore
            .insertOrMergeTableEntity(
                &context,
                &table,
                &account,
                persisted,
                if_match.as_deref(),
                table_context.batchId().as_deref(),
            )
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        response.insert_field("eTag", string_value(persisted.eTag));
        Ok(response)
    }

    async fn deleteEntity(
        &self,
        options: TableDeleteEntityOptionalParams,
        context: Context,
    ) -> Result<TableDeleteEntityResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        let (partition_key, row_key) = self.get_required_entity_keys(&table_context, None)?;
        let if_match = get_string(&options, "ifMatch").or_else(|| {
            context
                .request()
                .and_then(|request| request.getHeader("if-match"))
        });
        let if_match =
            if_match.ok_or_else(|| StorageErrorFactory::getPreconditionFailed(&context))?;
        if if_match.is_empty() {
            return Err(StorageErrorFactory::getPreconditionFailed(&context));
        }
        if if_match != "*" && isEtagValid(&if_match) {
            return Err(StorageErrorFactory::getInvalidInput(&context, None));
        }

        self.base
            .metadataStore
            .deleteTableEntity(
                &context,
                &table,
                &account,
                &partition_key,
                &row_key,
                &if_match,
                table_context.batchId().as_deref(),
            )
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn insertEntity(
        &self,
        mut entity: TableEntity,
        options: TableInsertEntityOptionalParams,
        context: Context,
    ) -> Result<TableInsertEntityResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        let accept = self.base.get_and_check_payload_format(&context)?;
        let prefer = self.base.get_prefer_header(&context);
        self.check_body_limit(
            &context,
            context
                .request()
                .and_then(|request| request.getBody())
                .as_deref(),
        )?;

        let (partition_key, row_key) =
            self.get_required_entity_keys(&table_context, Some(&entity))?;
        self.validate_key(&context, &partition_key)?;
        self.validate_key(&context, &row_key)?;
        self.check_properties(&context, &entity)?;
        self.remove_etag_property(&mut entity);

        let persisted = self.create_persisted_entity(
            &context,
            Some(entity),
            partition_key.clone(),
            row_key.clone(),
        );
        self.validate_entity_shape(&context, &persisted)?;
        let persisted = self
            .base
            .metadataStore
            .insertTableEntity(
                &context,
                &table,
                &account,
                persisted,
                table_context.batchId().as_deref(),
            )
            .await?;

        let mut response = GeneratedResponse::new(201);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        response.insert_field("eTag", string_value(persisted.eTag.clone()));

        if prefer.as_deref() != Some("return-no-content") {
            let body = self.build_entity_response_body(
                &persisted,
                &accept,
                self.entity_injection_fields(
                    &account,
                    &table,
                    &partition_key,
                    &row_key,
                    &accept,
                    &persisted.eTag,
                    &context,
                ),
                None,
                &context,
            )?;
            response.body = Some(BaseHandler::stream_body(body));
        }

        self.base.update_response_prefer(&mut response, &context);
        self.base
            .set_response_content_type(&mut response, Some(&accept));
        Ok(response)
    }

    async fn getAccessPolicy(
        &self,
        options: TableGetAccessPolicyOptionalParams,
        context: Context,
    ) -> Result<TableGetAccessPolicyResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;

        let found_table = self
            .base
            .metadataStore
            .getTable(&account, &table, &context)
            .await?
            .ok_or_else(|| StorageErrorFactory::getTableNotFound(&context))?;

        let body = found_table
            .tableAcl
            .unwrap_or_default()
            .into_iter()
            .map(|identifier| {
                let mut access_policy = GeneratedObject::new();
                access_policy.insert(
                    String::from("start"),
                    string_value(identifier.accessPolicy.start),
                );
                access_policy.insert(
                    String::from("expiry"),
                    string_value(identifier.accessPolicy.expiry),
                );
                access_policy.insert(
                    String::from("permission"),
                    string_value(identifier.accessPolicy.permission),
                );
                let mut object = GeneratedObject::new();
                object.insert(String::from("id"), string_value(identifier.id));
                object.insert(
                    String::from("accessPolicy"),
                    GeneratedValue::Object(access_policy),
                );
                GeneratedValue::Object(object)
            })
            .collect();

        let mut response = GeneratedResponse::new(200);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        response.body = Some(GeneratedBody::Value(GeneratedValue::Array(body)));
        Ok(response)
    }

    async fn setAccessPolicy(
        &self,
        signed_identifiers: Vec<SignedIdentifier>,
        options: TableSetAccessPolicyOptionalParams,
        context: Context,
    ) -> Result<TableSetAccessPolicyResponse, crate::errors::StorageError> {
        let table_context = TableStorageContext::new(&context);
        let account = self.account_name(&table_context)?;
        let table = self.table_name(&table_context)?;
        self.check_body_limit(
            &context,
            context
                .request()
                .and_then(|request| request.getBody())
                .as_deref(),
        )?;

        let table_acl = if signed_identifiers.is_empty() {
            None
        } else {
            if signed_identifiers.len() > 5 {
                return Err(StorageErrorFactory::getInvalidXmlDocument(&context));
            }
            let mut acl = Vec::with_capacity(signed_identifiers.len());
            for identifier in signed_identifiers {
                let Some(id) = get_string(&identifier, "id") else {
                    return Err(StorageErrorFactory::getInvalidXmlDocument(&context));
                };
                let access_policy = identifier
                    .get("accessPolicy")
                    .and_then(GeneratedValue::as_object)
                    .ok_or_else(|| StorageErrorFactory::getInvalidXmlDocument(&context))?;
                let start = get_string(access_policy, "start")
                    .ok_or_else(|| StorageErrorFactory::getInvalidXmlDocument(&context))?;
                let expiry = get_string(access_policy, "expiry")
                    .ok_or_else(|| StorageErrorFactory::getInvalidXmlDocument(&context))?;
                let permission = get_string(access_policy, "permission")
                    .ok_or_else(|| StorageErrorFactory::getInvalidXmlDocument(&context))?;
                if permission
                    .chars()
                    .any(|item| !TABLE_SERVICE_PERMISSION.contains(item))
                {
                    return Err(StorageErrorFactory::getInvalidXmlDocument(&context));
                }
                acl.push(PersistedSignedIdentifier {
                    id,
                    accessPolicy: AccessPolicy {
                        start,
                        expiry,
                        permission,
                    },
                });
            }
            Some(acl)
        };

        self.base
            .metadataStore
            .setTableACL(&account, &table, &context, table_acl)
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }
}

fn add_optional(object: &mut GeneratedObject, key: &str, value: Option<String>) {
    if let Some(value) = value {
        object.insert(key.to_string(), string_value(value));
    }
}
