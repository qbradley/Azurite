use std::collections::HashMap;
use std::sync::LazyLock;

use azurite_common::authentication::{
    AccountSASPermission, AccountSASResourceType, AccountSASService,
};

use crate::generated::artifacts::operation::Operation;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationAccountSASPermission {
    pub service: String,
    pub resourceType: String,
    pub permission: String,
}

impl OperationAccountSASPermission {
    pub fn new(service: &str, resourceType: &str, permission: &str) -> Self {
        Self {
            service: String::from(service),
            resourceType: String::from(resourceType),
            permission: String::from(permission),
        }
    }

    pub fn validate(&self, services: &str, resourceTypes: &str, permissions: &str) -> bool {
        self.validateServices(services)
            && self.validateResourceTypes(resourceTypes)
            && self.validatePermissions(permissions)
    }

    #[allow(non_snake_case)]
    pub fn validateServices(&self, services: &str) -> bool {
        services.contains(&self.service)
    }

    #[allow(non_snake_case)]
    pub fn validateResourceTypes(&self, resourceTypes: &str) -> bool {
        for resource_type in self.resourceType.chars() {
            if resourceTypes.contains(resource_type) {
                return true;
            }
        }
        false
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

pub static OPERATION_ACCOUNT_SAS_PERMISSIONS: LazyLock<
    HashMap<Operation, OperationAccountSASPermission>,
> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(
        Operation::Service_GetProperties,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Service.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Service_SetProperties,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Service.as_str(),
            AccountSASPermission::Write.as_str(),
        ),
    );
    map.insert(
        Operation::Service_ListQueuesSegment,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Service.as_str(),
            AccountSASPermission::List.as_str(),
        ),
    );
    map.insert(
        Operation::Service_GetStatistics,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Service.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Queue_Create,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            &format!(
                "{}{}",
                AccountSASPermission::Create.as_str(),
                AccountSASPermission::Write.as_str()
            ),
        ),
    );
    map.insert(
        Operation::Queue_Delete,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Delete.as_str(),
        ),
    );
    map.insert(
        Operation::Queue_GetProperties,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Queue_GetPropertiesWithHead,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Queue_SetMetadata,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Write.as_str(),
        ),
    );
    map.insert(
        Operation::Queue_GetAccessPolicy,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            "",
        ),
    );
    map.insert(
        Operation::Queue_GetAccessPolicyWithHead,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            "",
        ),
    );
    map.insert(
        Operation::Queue_SetAccessPolicy,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            "",
        ),
    );
    map.insert(
        Operation::Messages_Enqueue,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Add.as_str(),
        ),
    );
    map.insert(
        Operation::Messages_Dequeue,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Process.as_str(),
        ),
    );
    map.insert(
        Operation::Messages_Peek,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::MessageId_Delete,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Process.as_str(),
        ),
    );
    map.insert(
        Operation::Messages_Clear,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Delete.as_str(),
        ),
    );
    map.insert(
        Operation::MessageId_Update,
        OperationAccountSASPermission::new(
            AccountSASService::Queue.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Update.as_str(),
        ),
    );

    map
});

#[cfg(test)]
mod tests {
    use crate::generated::artifacts::operation::Operation;

    use super::OPERATION_ACCOUNT_SAS_PERMISSIONS;

    #[test]
    fn account_sas_permission_map_matches_queue_service() {
        let permissions = &OPERATION_ACCOUNT_SAS_PERMISSIONS[&Operation::Messages_Dequeue];
        assert!(permissions.validate("q", "o", "p"));
        assert!(!permissions.validate("b", "o", "p"));
        assert!(!permissions.validate("q", "c", "p"));
        assert!(!permissions.validate("q", "o", "r"));

        let acl_permissions = &OPERATION_ACCOUNT_SAS_PERMISSIONS[&Operation::Queue_GetAccessPolicy];
        assert!(acl_permissions.validateServices("q"));
        assert!(acl_permissions.validateResourceTypes("c"));
        assert!(!acl_permissions.validatePermissions("rwdlcaup"));
    }
}
