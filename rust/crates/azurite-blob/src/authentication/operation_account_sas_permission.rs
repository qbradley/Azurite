use std::collections::HashMap;
use std::sync::LazyLock;

use azurite_common::authentication::{AccountSASPermission, AccountSASResourceType};

use crate::generated::artifacts::operation::Operation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationAccountSASPermission {
    pub service: String,
    pub resourceType: String,
    pub permission: String,
}

#[allow(non_snake_case)]
impl OperationAccountSASPermission {
    pub fn new(service: &str, resourceType: &str, permission: &str) -> Self {
        Self {
            service: String::from(service),
            resourceType: String::from(resourceType),
            permission: String::from(permission),
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

    pub fn validateServices<T: ToString>(&self, services: T) -> bool {
        services.to_string().contains(&self.service)
    }

    pub fn validateResourceTypes<T: ToString>(&self, resourceTypes: T) -> bool {
        let resourceTypes = resourceTypes.to_string();
        if self.resourceType == AccountSASResourceType::Any.as_str() {
            return !resourceTypes.is_empty();
        }
        for p in self.resourceType.chars() {
            if resourceTypes.contains(p) {
                return true;
            }
        }
        false
    }

    pub fn validatePermissions<T: ToString>(&self, permissions: T) -> bool {
        let permissions = permissions.to_string();
        if self.permission == AccountSASPermission::Any.as_str() {
            return !permissions.is_empty();
        }
        for p in self.permission.chars() {
            if permissions.contains(p) {
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
        Operation::Service_GetAccountInfo,
        OperationAccountSASPermission::new("b", "sco", "rcdlpruw"),
    );
    map.insert(
        Operation::Service_GetAccountInfoWithHead,
        OperationAccountSASPermission::new("b", "sco", "rcdlpruw"),
    );
    map.insert(
        Operation::Container_GetAccountInfo,
        OperationAccountSASPermission::new("b", "sco", "rcdlpruw"),
    );
    map.insert(
        Operation::Container_GetAccountInfoWithHead,
        OperationAccountSASPermission::new("b", "sco", "rcdlpruw"),
    );
    map.insert(
        Operation::Blob_GetAccountInfo,
        OperationAccountSASPermission::new("b", "sco", "rcdlpruw"),
    );
    map.insert(
        Operation::Blob_GetAccountInfoWithHead,
        OperationAccountSASPermission::new("b", "sco", "rcdlpruw"),
    );
    map.insert(
        Operation::Service_ListContainersSegment,
        OperationAccountSASPermission::new("b", "s", "l"),
    );
    map.insert(
        Operation::Service_GetProperties,
        OperationAccountSASPermission::new("b", "s", "r"),
    );
    map.insert(
        Operation::Service_SetProperties,
        OperationAccountSASPermission::new("b", "s", "w"),
    );
    map.insert(
        Operation::Service_SubmitBatch,
        OperationAccountSASPermission::new("b", "AnyResourceType", "AnyPermission"),
    );
    map.insert(
        Operation::Service_GetStatistics,
        OperationAccountSASPermission::new("b", "s", "r"),
    );
    map.insert(
        Operation::Container_Create,
        OperationAccountSASPermission::new("b", "c", "cw"),
    );
    map.insert(
        Operation::Container_SetAccessPolicy,
        OperationAccountSASPermission::new("b", "c", ""),
    );
    map.insert(
        Operation::Container_GetAccessPolicy,
        OperationAccountSASPermission::new("b", "c", ""),
    );
    map.insert(
        Operation::Container_GetProperties,
        OperationAccountSASPermission::new("b", "c", "r"),
    );
    map.insert(
        Operation::Container_GetPropertiesWithHead,
        OperationAccountSASPermission::new("b", "c", "r"),
    );
    map.insert(
        Operation::Container_SetMetadata,
        OperationAccountSASPermission::new("b", "c", "w"),
    );
    map.insert(
        Operation::Container_BreakLease,
        OperationAccountSASPermission::new("b", "c", "wd"),
    );
    map.insert(
        Operation::Container_RenewLease,
        OperationAccountSASPermission::new("b", "c", "w"),
    );
    map.insert(
        Operation::Container_ChangeLease,
        OperationAccountSASPermission::new("b", "c", "w"),
    );
    map.insert(
        Operation::Container_AcquireLease,
        OperationAccountSASPermission::new("b", "c", "w"),
    );
    map.insert(
        Operation::Container_ReleaseLease,
        OperationAccountSASPermission::new("b", "c", "w"),
    );
    map.insert(
        Operation::Container_Delete,
        OperationAccountSASPermission::new("b", "c", "d"),
    );
    map.insert(
        Operation::Container_ListBlobHierarchySegment,
        OperationAccountSASPermission::new("b", "c", "l"),
    );
    map.insert(
        Operation::Container_ListBlobFlatSegment,
        OperationAccountSASPermission::new("b", "c", "l"),
    );
    map.insert(
        Operation::BlockBlob_Upload,
        OperationAccountSASPermission::new("b", "o", "wc"),
    );
    map.insert(
        Operation::PageBlob_Create,
        OperationAccountSASPermission::new("b", "o", "wc"),
    );
    map.insert(
        Operation::AppendBlob_Create,
        OperationAccountSASPermission::new("b", "o", "wc"),
    );
    map.insert(
        Operation::Blob_Download,
        OperationAccountSASPermission::new("b", "o", "r"),
    );
    map.insert(
        Operation::Blob_GetProperties,
        OperationAccountSASPermission::new("b", "o", "r"),
    );
    map.insert(
        Operation::Blob_SetHTTPHeaders,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_GetProperties,
        OperationAccountSASPermission::new("b", "o", "r"),
    );
    map.insert(
        Operation::Blob_SetMetadata,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_Delete,
        OperationAccountSASPermission::new("b", "o", "d"),
    );
    map.insert(
        Operation::Blob_BreakLease,
        OperationAccountSASPermission::new("b", "o", "dw"),
    );
    map.insert(
        Operation::Blob_RenewLease,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_ChangeLease,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_AcquireLease,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_ReleaseLease,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_CreateSnapshot,
        OperationAccountSASPermission::new("b", "o", "wc"),
    );
    map.insert(
        Operation::Blob_StartCopyFromURL,
        OperationAccountSASPermission::new("b", "o", "wc"),
    );
    map.insert(
        Operation::Blob_CopyFromURL,
        OperationAccountSASPermission::new("b", "o", "wc"),
    );
    map.insert(
        Operation::PageBlob_CopyIncremental,
        OperationAccountSASPermission::new("b", "o", "wc"),
    );
    map.insert(
        Operation::Blob_AbortCopyFromURL,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::BlockBlob_StageBlock,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::BlockBlob_CommitBlockList,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::BlockBlob_GetBlockList,
        OperationAccountSASPermission::new("b", "o", "r"),
    );
    map.insert(
        Operation::PageBlob_UploadPages,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::PageBlob_GetPageRanges,
        OperationAccountSASPermission::new("b", "o", "r"),
    );
    map.insert(
        Operation::PageBlob_GetPageRangesDiff,
        OperationAccountSASPermission::new("b", "o", "r"),
    );
    map.insert(
        Operation::AppendBlob_AppendBlock,
        OperationAccountSASPermission::new("b", "o", "aw"),
    );
    map.insert(
        Operation::PageBlob_ClearPages,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_SetTier,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::PageBlob_UpdateSequenceNumber,
        OperationAccountSASPermission::new("b", "o", "w"),
    );
    map.insert(
        Operation::Blob_SetTags,
        OperationAccountSASPermission::new("b", "o", "t"),
    );
    map.insert(
        Operation::Blob_GetTags,
        OperationAccountSASPermission::new("b", "o", "t"),
    );
    map.insert(
        Operation::Service_FilterBlobs,
        OperationAccountSASPermission::new("b", "o", "f"),
    );
    map.insert(
        Operation::Container_FilterBlobs,
        OperationAccountSASPermission::new("b", "c", "f"),
    );
    map
});
