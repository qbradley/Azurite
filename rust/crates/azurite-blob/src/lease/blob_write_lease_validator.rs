/// Mirrors TypeScript `src/blob/lease/BlobWriteLeaseValidator.ts`.
///
/// Fidelity:
/// - Requires non-empty leaseId when blob is `Locked`; stale leaseId on
///   unlocked blob ⇒ `BlobLeaseLost`.
use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::errors::StorageError;
use crate::generated::artifacts::models::LeaseAccessConditions;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{ILease, ILeaseValidator, LeaseStatusType};

pub struct BlobWriteLeaseValidator {
    pub leaseAccessConditions: Option<LeaseAccessConditions>,
}

impl BlobWriteLeaseValidator {
    pub fn new(lease_access_conditions: Option<LeaseAccessConditions>) -> Self {
        Self {
            leaseAccessConditions: lease_access_conditions,
        }
    }
}

impl ILeaseValidator for BlobWriteLeaseValidator {
    fn validate(&self, lease: &ILease, context: &Context) -> Result<(), StorageError> {
        let input_lease_id = self
            .leaseAccessConditions
            .as_ref()
            .and_then(|c| c.get("leaseId"))
            .and_then(|v| v.as_string())
            .filter(|s| !s.is_empty());

        if lease.leaseStatus.as_deref() == Some(LeaseStatusType::Locked) {
            if input_lease_id.is_none() {
                return Err(StorageErrorFactory::getBlobLeaseIdMissing(
                    context.contextId().as_deref(),
                ));
            } else if let Some(input_id) = &input_lease_id {
                if let Some(stored_id) = &lease.leaseId {
                    if input_id.to_lowercase() != stored_id.to_lowercase() {
                        return Err(
                            StorageErrorFactory::getBlobLeaseIdMismatchWithBlobOperation(
                                context.contextId().as_deref(),
                            ),
                        );
                    }
                }
            }
        } else if input_lease_id.is_some() {
            // Stale leaseId provided on an unlocked blob ⇒ BlobLeaseLost
            return Err(StorageErrorFactory::getBlobLeaseLost(
                context.contextId().as_deref(),
            ));
        }

        Ok(())
    }
}
