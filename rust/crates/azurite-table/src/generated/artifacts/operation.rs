#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum Operation {
    Table_Query = 0,
    Table_Create = 1,
    Table_Batch = 2,
    Table_Delete = 3,
    Table_QueryEntities = 4,
    Table_QueryEntitiesWithPartitionAndRowKey = 5,
    Table_UpdateEntity = 6,
    Table_MergeEntity = 7,
    Table_DeleteEntity = 8,
    Table_MergeEntityWithMerge = 9,
    Table_InsertEntity = 10,
    Table_GetAccessPolicy = 11,
    Table_SetAccessPolicy = 12,
    Service_SetProperties = 13,
    Service_GetProperties = 14,
    Service_GetStatistics = 15,
}

pub const ALL_OPERATIONS: [Operation; 16] = [
    Operation::Table_Query,
    Operation::Table_Create,
    Operation::Table_Batch,
    Operation::Table_Delete,
    Operation::Table_QueryEntities,
    Operation::Table_QueryEntitiesWithPartitionAndRowKey,
    Operation::Table_UpdateEntity,
    Operation::Table_MergeEntity,
    Operation::Table_DeleteEntity,
    Operation::Table_MergeEntityWithMerge,
    Operation::Table_InsertEntity,
    Operation::Table_GetAccessPolicy,
    Operation::Table_SetAccessPolicy,
    Operation::Service_SetProperties,
    Operation::Service_GetProperties,
    Operation::Service_GetStatistics,
];

impl Operation {
    pub fn as_usize(self) -> usize {
        self as usize
    }

    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Operation::Table_Query),
            1 => Some(Operation::Table_Create),
            2 => Some(Operation::Table_Batch),
            3 => Some(Operation::Table_Delete),
            4 => Some(Operation::Table_QueryEntities),
            5 => Some(Operation::Table_QueryEntitiesWithPartitionAndRowKey),
            6 => Some(Operation::Table_UpdateEntity),
            7 => Some(Operation::Table_MergeEntity),
            8 => Some(Operation::Table_DeleteEntity),
            9 => Some(Operation::Table_MergeEntityWithMerge),
            10 => Some(Operation::Table_InsertEntity),
            11 => Some(Operation::Table_GetAccessPolicy),
            12 => Some(Operation::Table_SetAccessPolicy),
            13 => Some(Operation::Service_SetProperties),
            14 => Some(Operation::Service_GetProperties),
            15 => Some(Operation::Service_GetStatistics),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Operation::Table_Query => "Table_Query",
            Operation::Table_Create => "Table_Create",
            Operation::Table_Batch => "Table_Batch",
            Operation::Table_Delete => "Table_Delete",
            Operation::Table_QueryEntities => "Table_QueryEntities",
            Operation::Table_QueryEntitiesWithPartitionAndRowKey => {
                "Table_QueryEntitiesWithPartitionAndRowKey"
            }
            Operation::Table_UpdateEntity => "Table_UpdateEntity",
            Operation::Table_MergeEntity => "Table_MergeEntity",
            Operation::Table_DeleteEntity => "Table_DeleteEntity",
            Operation::Table_MergeEntityWithMerge => "Table_MergeEntityWithMerge",
            Operation::Table_InsertEntity => "Table_InsertEntity",
            Operation::Table_GetAccessPolicy => "Table_GetAccessPolicy",
            Operation::Table_SetAccessPolicy => "Table_SetAccessPolicy",
            Operation::Service_SetProperties => "Service_SetProperties",
            Operation::Service_GetProperties => "Service_GetProperties",
            Operation::Service_GetStatistics => "Service_GetStatistics",
        }
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
