#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum Operation {
    Service_SetProperties = 0,
    Service_GetProperties = 1,
    Service_GetStatistics = 2,
    Service_ListQueuesSegment = 3,
    Queue_Create = 4,
    Queue_Delete = 5,
    Queue_GetProperties = 6,
    Queue_GetPropertiesWithHead = 7,
    Queue_SetMetadata = 8,
    Queue_GetAccessPolicy = 9,
    Queue_GetAccessPolicyWithHead = 10,
    Queue_SetAccessPolicy = 11,
    Messages_Dequeue = 12,
    Messages_Clear = 13,
    Messages_Enqueue = 14,
    Messages_Peek = 15,
    MessageId_Update = 16,
    MessageId_Delete = 17,
}

pub const ALL_OPERATIONS: [Operation; 18] = [
    Operation::Service_SetProperties,
    Operation::Service_GetProperties,
    Operation::Service_GetStatistics,
    Operation::Service_ListQueuesSegment,
    Operation::Queue_Create,
    Operation::Queue_Delete,
    Operation::Queue_GetProperties,
    Operation::Queue_GetPropertiesWithHead,
    Operation::Queue_SetMetadata,
    Operation::Queue_GetAccessPolicy,
    Operation::Queue_GetAccessPolicyWithHead,
    Operation::Queue_SetAccessPolicy,
    Operation::Messages_Dequeue,
    Operation::Messages_Clear,
    Operation::Messages_Enqueue,
    Operation::Messages_Peek,
    Operation::MessageId_Update,
    Operation::MessageId_Delete,
];

impl Operation {
    pub fn as_usize(self) -> usize {
        self as usize
    }

    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Operation::Service_SetProperties),
            1 => Some(Operation::Service_GetProperties),
            2 => Some(Operation::Service_GetStatistics),
            3 => Some(Operation::Service_ListQueuesSegment),
            4 => Some(Operation::Queue_Create),
            5 => Some(Operation::Queue_Delete),
            6 => Some(Operation::Queue_GetProperties),
            7 => Some(Operation::Queue_GetPropertiesWithHead),
            8 => Some(Operation::Queue_SetMetadata),
            9 => Some(Operation::Queue_GetAccessPolicy),
            10 => Some(Operation::Queue_GetAccessPolicyWithHead),
            11 => Some(Operation::Queue_SetAccessPolicy),
            12 => Some(Operation::Messages_Dequeue),
            13 => Some(Operation::Messages_Clear),
            14 => Some(Operation::Messages_Enqueue),
            15 => Some(Operation::Messages_Peek),
            16 => Some(Operation::MessageId_Update),
            17 => Some(Operation::MessageId_Delete),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Operation::Service_SetProperties => "Service_SetProperties",
            Operation::Service_GetProperties => "Service_GetProperties",
            Operation::Service_GetStatistics => "Service_GetStatistics",
            Operation::Service_ListQueuesSegment => "Service_ListQueuesSegment",
            Operation::Queue_Create => "Queue_Create",
            Operation::Queue_Delete => "Queue_Delete",
            Operation::Queue_GetProperties => "Queue_GetProperties",
            Operation::Queue_GetPropertiesWithHead => "Queue_GetPropertiesWithHead",
            Operation::Queue_SetMetadata => "Queue_SetMetadata",
            Operation::Queue_GetAccessPolicy => "Queue_GetAccessPolicy",
            Operation::Queue_GetAccessPolicyWithHead => "Queue_GetAccessPolicyWithHead",
            Operation::Queue_SetAccessPolicy => "Queue_SetAccessPolicy",
            Operation::Messages_Dequeue => "Messages_Dequeue",
            Operation::Messages_Clear => "Messages_Clear",
            Operation::Messages_Enqueue => "Messages_Enqueue",
            Operation::Messages_Peek => "Messages_Peek",
            Operation::MessageId_Update => "MessageId_Update",
            Operation::MessageId_Delete => "MessageId_Delete",
        }
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
