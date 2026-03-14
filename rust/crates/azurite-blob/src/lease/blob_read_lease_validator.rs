/// Mirrors TypeScript `src/blob/lease/BlobReadLeaseValidator.ts`.
///
/// Fidelity: only acts when a non-empty leaseId was supplied in
/// `leaseAccessConditions`.
use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::errors::StorageError;
use crate::generated::artifacts::models::LeaseAccessConditions;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{ILease, ILeaseValidator, LeaseStatusType};

pub struct BlobReadLeaseValidator {
    pub leaseAccessConditions: Option<LeaseAccessConditions>,
}

impl BlobReadLeaseValidator {
    pub fn new(lease_access_conditions: Option<LeaseAccessConditions>) -> Self {
        Self {
            leaseAccessConditions: lease_access_conditions,
        }
    }
}

impl ILeaseValidator for BlobReadLeaseValidator {
    fn validate(&self, lease: &ILease, context: &Context) -> Result<(), StorageError> {
        // Check only when input leaseId is not empty
        let input_lease_id = self
            .leaseAccessConditions
            .as_ref()
            .and_then(|c| c.get("leaseId"))
            .and_then(|v| v.as_string())
            .filter(|s| !s.is_empty());

        if let Some(input_id) = input_lease_id {
            if lease.leaseStatus.as_deref() == Some(LeaseStatusType::Unlocked) {
                return Err(StorageErrorFactory::getBlobLeaseLost(
                    context.contextId().as_deref(),
                ));
            } else if let Some(stored_id) = &lease.leaseId {
                if input_id.to_lowercase() != stored_id.to_lowercase() {
                    return Err(
                        StorageErrorFactory::getBlobLeaseIdMismatchWithBlobOperation(
                            context.contextId().as_deref(),
                        ),
                    );
                }
            }
        }

        Ok(())
    }
}
