use std::collections::HashMap;
use std::sync::LazyLock;

use crate::generated::artifacts::operation::Operation;

use super::table_sas_permissions::TableSASPermission;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationTableSASPermission {
    pub permission: String,
}

impl OperationTableSASPermission {
    pub fn new(permission: impl Into<String>) -> Self {
        Self {
            permission: permission.into(),
        }
    }

    pub fn validate<T: ToString>(&self, permissions: T) -> bool {
        self.validatePermissions(permissions)
    }

    #[allow(non_snake_case)]
    pub fn validatePermissions<T: ToString>(&self, permissions: T) -> bool {
        let permissions = permissions.to_string();
        for permission in self.permission.chars() {
            if permissions.contains(permission) {
                return true;
            }
        }
        false
    }
}

pub static OPERATION_TABLE_SAS_TABLE_PERMISSIONS: LazyLock<
    HashMap<Operation, OperationTableSASPermission>,
> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(
        Operation::Service_SetProperties,
        OperationTableSASPermission::new(""),
    );
    map.insert(
        Operation::Service_GetProperties,
        OperationTableSASPermission::new(TableSASPermission::Query.as_str()),
    );
    map.insert(
        Operation::Service_GetStatistics,
        OperationTableSASPermission::new(TableSASPermission::Query.as_str()),
    );
    map.insert(
        Operation::Table_Query,
        OperationTableSASPermission::new(TableSASPermission::Query.as_str()),
    );
    map.insert(
        Operation::Table_Create,
        OperationTableSASPermission::new(""),
    );
    map.insert(
        Operation::Table_Delete,
        OperationTableSASPermission::new(TableSASPermission::Delete.as_str()),
    );
    map.insert(
        Operation::Table_QueryEntities,
        OperationTableSASPermission::new(TableSASPermission::Query.as_str()),
    );
    map.insert(
        Operation::Table_QueryEntitiesWithPartitionAndRowKey,
        OperationTableSASPermission::new(TableSASPermission::Query.as_str()),
    );
    map.insert(
        Operation::Table_UpdateEntity,
        OperationTableSASPermission::new(TableSASPermission::Update.as_str()),
    );
    map.insert(
        Operation::Table_MergeEntity,
        OperationTableSASPermission::new(TableSASPermission::Update.as_str()),
    );
    map.insert(
        Operation::Table_DeleteEntity,
        OperationTableSASPermission::new(TableSASPermission::Delete.as_str()),
    );
    map.insert(
        Operation::Table_MergeEntityWithMerge,
        OperationTableSASPermission::new(TableSASPermission::Update.as_str()),
    );
    map.insert(
        Operation::Table_InsertEntity,
        OperationTableSASPermission::new(TableSASPermission::Add.as_str()),
    );
    map.insert(
        Operation::Table_GetAccessPolicy,
        OperationTableSASPermission::new(""),
    );
    map.insert(
        Operation::Table_SetAccessPolicy,
        OperationTableSASPermission::new(""),
    );
    map.insert(
        Operation::Table_Batch,
        OperationTableSASPermission::new(format!(
            "{}{}{}{}",
            TableSASPermission::Add,
            TableSASPermission::Delete,
            TableSASPermission::Query,
            TableSASPermission::Update,
        )),
    );
    map
});

#[cfg(test)]
mod tests {
    use crate::generated::artifacts::operation::Operation;

    use super::OPERATION_TABLE_SAS_TABLE_PERMISSIONS;

    #[test]
    fn table_batch_permissions_use_any_matching_semantics() {
        let permission = &OPERATION_TABLE_SAS_TABLE_PERMISSIONS[&Operation::Table_Batch];

        assert!(permission.validatePermissions("a"));
        assert!(permission.validatePermissions("u"));
        assert!(!permission.validatePermissions("c"));
    }
}
