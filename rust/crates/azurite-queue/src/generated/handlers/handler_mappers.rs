use crate::generated::artifacts::operation::Operation;

#[derive(Debug, Clone, Copy)]
pub struct HandlerPath {
    pub handler: &'static str,
    pub method: &'static str,
    pub arguments: &'static [&'static str],
}

const ARGUMENTS_0: &[&str] = &["storageServiceProperties", "options"];
const ARGUMENTS_1: &[&str] = &["options"];
const ARGUMENTS_2: &[&str] = &["options"];
const ARGUMENTS_3: &[&str] = &["options"];
const ARGUMENTS_4: &[&str] = &["options"];
const ARGUMENTS_5: &[&str] = &["options"];
const ARGUMENTS_6: &[&str] = &["options"];
const ARGUMENTS_7: &[&str] = &["options"];
const ARGUMENTS_8: &[&str] = &["options"];
const ARGUMENTS_9: &[&str] = &["options"];
const ARGUMENTS_10: &[&str] = &["options"];
const ARGUMENTS_11: &[&str] = &["options"];
const ARGUMENTS_12: &[&str] = &["options"];
const ARGUMENTS_13: &[&str] = &["options"];
const ARGUMENTS_14: &[&str] = &["queueMessage", "options"];
const ARGUMENTS_15: &[&str] = &["options"];
const ARGUMENTS_16: &[&str] = &["queueMessage", "popReceipt", "visibilitytimeout", "options"];
const ARGUMENTS_17: &[&str] = &["popReceipt", "options"];

pub fn getHandlerByOperation(operation: Operation) -> HandlerPath {
    match operation {
        Operation::Service_SetProperties => HandlerPath {
            handler: "serviceHandler",
            method: "setProperties",
            arguments: ARGUMENTS_0,
        },
        Operation::Service_GetProperties => HandlerPath {
            handler: "serviceHandler",
            method: "getProperties",
            arguments: ARGUMENTS_1,
        },
        Operation::Service_GetStatistics => HandlerPath {
            handler: "serviceHandler",
            method: "getStatistics",
            arguments: ARGUMENTS_2,
        },
        Operation::Service_ListQueuesSegment => HandlerPath {
            handler: "serviceHandler",
            method: "listQueuesSegment",
            arguments: ARGUMENTS_3,
        },
        Operation::Queue_Create => HandlerPath {
            handler: "queueHandler",
            method: "create",
            arguments: ARGUMENTS_4,
        },
        Operation::Queue_Delete => HandlerPath {
            handler: "queueHandler",
            method: "delete",
            arguments: ARGUMENTS_5,
        },
        Operation::Queue_GetProperties => HandlerPath {
            handler: "queueHandler",
            method: "getProperties",
            arguments: ARGUMENTS_6,
        },
        Operation::Queue_GetPropertiesWithHead => HandlerPath {
            handler: "queueHandler",
            method: "getPropertiesWithHead",
            arguments: ARGUMENTS_7,
        },
        Operation::Queue_SetMetadata => HandlerPath {
            handler: "queueHandler",
            method: "setMetadata",
            arguments: ARGUMENTS_8,
        },
        Operation::Queue_GetAccessPolicy => HandlerPath {
            handler: "queueHandler",
            method: "getAccessPolicy",
            arguments: ARGUMENTS_9,
        },
        Operation::Queue_GetAccessPolicyWithHead => HandlerPath {
            handler: "queueHandler",
            method: "getAccessPolicyWithHead",
            arguments: ARGUMENTS_10,
        },
        Operation::Queue_SetAccessPolicy => HandlerPath {
            handler: "queueHandler",
            method: "setAccessPolicy",
            arguments: ARGUMENTS_11,
        },
        Operation::Messages_Dequeue => HandlerPath {
            handler: "messagesHandler",
            method: "dequeue",
            arguments: ARGUMENTS_12,
        },
        Operation::Messages_Clear => HandlerPath {
            handler: "messagesHandler",
            method: "clear",
            arguments: ARGUMENTS_13,
        },
        Operation::Messages_Enqueue => HandlerPath {
            handler: "messagesHandler",
            method: "enqueue",
            arguments: ARGUMENTS_14,
        },
        Operation::Messages_Peek => HandlerPath {
            handler: "messagesHandler",
            method: "peek",
            arguments: ARGUMENTS_15,
        },
        Operation::MessageId_Update => HandlerPath {
            handler: "messageIdHandler",
            method: "update",
            arguments: ARGUMENTS_16,
        },
        Operation::MessageId_Delete => HandlerPath {
            handler: "messageIdHandler",
            method: "delete",
            arguments: ARGUMENTS_17,
        },
    }
}
