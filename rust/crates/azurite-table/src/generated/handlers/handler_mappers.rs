use crate::generated::artifacts::operation::Operation;

#[derive(Debug, Clone, Copy)]
pub struct HandlerPath {
    pub handler: &'static str,
    pub method: &'static str,
    pub arguments: &'static [&'static str],
}

const ARGUMENTS_0: &[&str] = &["options"];
const ARGUMENTS_1: &[&str] = &["tableProperties", "options"];
const ARGUMENTS_2: &[&str] = &["body", "multipartContentType", "contentLength", "options"];
const ARGUMENTS_3: &[&str] = &["table", "options"];
const ARGUMENTS_4: &[&str] = &["table", "options"];
const ARGUMENTS_5: &[&str] = &["table", "partitionKey", "rowKey", "options"];
const ARGUMENTS_6: &[&str] = &["table", "partitionKey", "rowKey", "options"];
const ARGUMENTS_7: &[&str] = &["table", "partitionKey", "rowKey", "options"];
const ARGUMENTS_8: &[&str] = &["table", "partitionKey", "rowKey", "ifMatch", "options"];
const ARGUMENTS_9: &[&str] = &["table", "partitionKey", "rowKey", "options"];
const ARGUMENTS_10: &[&str] = &["table", "options"];
const ARGUMENTS_11: &[&str] = &["table", "options"];
const ARGUMENTS_12: &[&str] = &["table", "options"];
const ARGUMENTS_13: &[&str] = &["tableServiceProperties", "options"];
const ARGUMENTS_14: &[&str] = &["options"];
const ARGUMENTS_15: &[&str] = &["options"];

pub fn getHandlerByOperation(operation: Operation) -> HandlerPath {
    match operation {
        Operation::Table_Query => HandlerPath {
            handler: "tableHandler",
            method: "query",
            arguments: ARGUMENTS_0,
        },
        Operation::Table_Create => HandlerPath {
            handler: "tableHandler",
            method: "create",
            arguments: ARGUMENTS_1,
        },
        Operation::Table_Batch => HandlerPath {
            handler: "tableHandler",
            method: "batch",
            arguments: ARGUMENTS_2,
        },
        Operation::Table_Delete => HandlerPath {
            handler: "tableHandler",
            method: "delete",
            arguments: ARGUMENTS_3,
        },
        Operation::Table_QueryEntities => HandlerPath {
            handler: "tableHandler",
            method: "queryEntities",
            arguments: ARGUMENTS_4,
        },
        Operation::Table_QueryEntitiesWithPartitionAndRowKey => HandlerPath {
            handler: "tableHandler",
            method: "queryEntitiesWithPartitionAndRowKey",
            arguments: ARGUMENTS_5,
        },
        Operation::Table_UpdateEntity => HandlerPath {
            handler: "tableHandler",
            method: "updateEntity",
            arguments: ARGUMENTS_6,
        },
        Operation::Table_MergeEntity => HandlerPath {
            handler: "tableHandler",
            method: "mergeEntity",
            arguments: ARGUMENTS_7,
        },
        Operation::Table_DeleteEntity => HandlerPath {
            handler: "tableHandler",
            method: "deleteEntity",
            arguments: ARGUMENTS_8,
        },
        Operation::Table_MergeEntityWithMerge => HandlerPath {
            handler: "tableHandler",
            method: "mergeEntityWithMerge",
            arguments: ARGUMENTS_9,
        },
        Operation::Table_InsertEntity => HandlerPath {
            handler: "tableHandler",
            method: "insertEntity",
            arguments: ARGUMENTS_10,
        },
        Operation::Table_GetAccessPolicy => HandlerPath {
            handler: "tableHandler",
            method: "getAccessPolicy",
            arguments: ARGUMENTS_11,
        },
        Operation::Table_SetAccessPolicy => HandlerPath {
            handler: "tableHandler",
            method: "setAccessPolicy",
            arguments: ARGUMENTS_12,
        },
        Operation::Service_SetProperties => HandlerPath {
            handler: "serviceHandler",
            method: "setProperties",
            arguments: ARGUMENTS_13,
        },
        Operation::Service_GetProperties => HandlerPath {
            handler: "serviceHandler",
            method: "getProperties",
            arguments: ARGUMENTS_14,
        },
        Operation::Service_GetStatistics => HandlerPath {
            handler: "serviceHandler",
            method: "getStatistics",
            arguments: ARGUMENTS_15,
        },
    }
}
