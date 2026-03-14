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
            service: service.to_owned(),
            resourceType: resourceType.to_owned(),
            permission: permission.to_owned(),
        }
    }

    pub fn validate<T1: ToString, T2: ToString, T3: ToString>(
        &self,
        services: T1,
        resourceTypes: T2,
        permissions: T3,
    ) -> bool {
        self.validateServices(services)
            && self.validateResourceTypes(resourceTypes)
            && self.validatePermissions(permissions)
    }

    #[allow(non_snake_case)]
    pub fn validateServices<T: ToString>(&self, services: T) -> bool {
        services.to_string().contains(&self.service)
    }

    #[allow(non_snake_case)]
    pub fn validateResourceTypes<T: ToString>(&self, resourceTypes: T) -> bool {
        let resourceTypes = resourceTypes.to_string();
        for resource_type in self.resourceType.chars() {
            if resourceTypes.contains(resource_type) {
                return true;
            }
        }
        false
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

pub static OPERATION_ACCOUNT_SAS_PERMISSIONS: LazyLock<
    HashMap<Operation, OperationAccountSASPermission>,
> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(
        Operation::Service_GetProperties,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Service.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Service_SetProperties,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Service.as_str(),
            AccountSASPermission::Write.as_str(),
        ),
    );
    map.insert(
        Operation::Service_GetStatistics,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Service.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Table_Query,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::List.as_str(),
        ),
    );
    map.insert(
        Operation::Table_Create,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Container.as_str(),
            &format!(
                "{}{}",
                AccountSASPermission::Create.as_str(),
                AccountSASPermission::Write.as_str(),
            ),
        ),
    );
    map.insert(
        Operation::Table_SetAccessPolicy,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Container.as_str(),
            "",
        ),
    );
    map.insert(
        Operation::Table_GetAccessPolicy,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Container.as_str(),
            "",
        ),
    );
    map.insert(
        Operation::Table_Delete,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Delete.as_str(),
        ),
    );
    map.insert(
        Operation::Table_QueryEntities,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Container.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Table_QueryEntitiesWithPartitionAndRowKey,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Read.as_str(),
        ),
    );
    map.insert(
        Operation::Table_InsertEntity,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Add.as_str(),
        ),
    );
    map.insert(
        Operation::Table_UpdateEntity,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Update.as_str(),
        ),
    );
    map.insert(
        Operation::Table_MergeEntity,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Update.as_str(),
        ),
    );
    map.insert(
        Operation::Table_MergeEntityWithMerge,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Update.as_str(),
        ),
    );
    map.insert(
        Operation::Table_DeleteEntity,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            AccountSASResourceType::Object.as_str(),
            AccountSASPermission::Delete.as_str(),
        ),
    );
    map.insert(
        Operation::Table_Batch,
        OperationAccountSASPermission::new(
            AccountSASService::Table.as_str(),
            &format!(
                "{}{}{}",
                AccountSASResourceType::Object.as_str(),
                AccountSASResourceType::Service.as_str(),
                AccountSASResourceType::Container.as_str(),
            ),
            &format!(
                "{}{}{}{}{}{}{}{}",
                AccountSASPermission::Delete.as_str(),
                AccountSASPermission::Add.as_str(),
                AccountSASPermission::Create.as_str(),
                AccountSASPermission::List.as_str(),
                AccountSASPermission::Process.as_str(),
                AccountSASPermission::Read.as_str(),
                AccountSASPermission::Update.as_str(),
                AccountSASPermission::Write.as_str(),
            ),
        ),
    );
    map
});

#[cfg(test)]
mod tests {
    use crate::generated::artifacts::operation::Operation;

    use super::OPERATION_ACCOUNT_SAS_PERMISSIONS;

    #[test]
    fn account_sas_permissions_use_any_matching_semantics() {
        let permission = &OPERATION_ACCOUNT_SAS_PERMISSIONS[&Operation::Table_Batch];

        assert!(permission.validate("t", "o", "u"));
        assert!(permission.validate("t", "s", "r"));
        assert!(!permission.validate("b", "o", "u"));
        assert!(!permission.validate("t", "x", "u"));
        assert!(!permission.validate("t", "o", "m"));
    }
}
