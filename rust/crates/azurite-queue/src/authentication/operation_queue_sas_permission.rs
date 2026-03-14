use std::collections::HashMap;
use std::sync::LazyLock;

use crate::generated::artifacts::operation::{Operation, ALL_OPERATIONS};

use super::queue_sas_permissions::QueueSASPermission;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationQueueSASPermission {
    pub permission: String,
}

impl OperationQueueSASPermission {
    pub fn new(permission: &str) -> Self {
        Self {
            permission: String::from(permission),
        }
    }

    pub fn validate(&self, permissions: &str) -> bool {
        self.validatePermissions(permissions)
    }

    #[allow(non_snake_case)]
    pub fn validatePermissions(&self, permissions: &str) -> bool {
        for permission in self.permission.chars() {
            if permissions.contains(permission) {
                return true;
            }
        }
        false
    }
}

pub static OPERATION_QUEUE_SAS_PERMISSIONS: LazyLock<
    HashMap<Operation, OperationQueueSASPermission>,
> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for operation in ALL_OPERATIONS {
        map.insert(operation, OperationQueueSASPermission::default());
    }

    map.insert(
        Operation::Queue_GetProperties,
        OperationQueueSASPermission::new(QueueSASPermission::Read.as_str()),
    );
    map.insert(
        Operation::Queue_GetPropertiesWithHead,
        OperationQueueSASPermission::new(QueueSASPermission::Read.as_str()),
    );
    map.insert(
        Operation::Messages_Peek,
        OperationQueueSASPermission::new(QueueSASPermission::Read.as_str()),
    );
    map.insert(
        Operation::Messages_Enqueue,
        OperationQueueSASPermission::new(QueueSASPermission::Add.as_str()),
    );
    map.insert(
        Operation::MessageId_Update,
        OperationQueueSASPermission::new(QueueSASPermission::Update.as_str()),
    );
    map.insert(
        Operation::Messages_Dequeue,
        OperationQueueSASPermission::new(QueueSASPermission::Process.as_str()),
    );
    map.insert(
        Operation::MessageId_Delete,
        OperationQueueSASPermission::new(QueueSASPermission::Process.as_str()),
    );
    map
});

#[cfg(test)]
mod tests {
    use crate::generated::artifacts::operation::Operation;

    use super::OPERATION_QUEUE_SAS_PERMISSIONS;

    #[test]
    fn queue_sas_permission_map_matches_queue_operations() {
        assert!(OPERATION_QUEUE_SAS_PERMISSIONS[&Operation::Queue_GetProperties].validate("r"));
        assert!(OPERATION_QUEUE_SAS_PERMISSIONS[&Operation::Messages_Enqueue].validate("a"));
        assert!(OPERATION_QUEUE_SAS_PERMISSIONS[&Operation::MessageId_Update].validate("u"));
        assert!(OPERATION_QUEUE_SAS_PERMISSIONS[&Operation::MessageId_Delete].validate("p"));
        assert!(!OPERATION_QUEUE_SAS_PERMISSIONS[&Operation::Queue_Create].validate("r"));
    }
}
