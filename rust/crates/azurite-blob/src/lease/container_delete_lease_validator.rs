/// Mirrors TypeScript `src/blob/lease/ContainerDeleteLeaseValidator.ts`.
///
/// Fidelity: the TS source checks `=== null` separately from `=== undefined` for
/// the `leaseId` field in `leaseAccessConditions`.  In Rust both absent and a
/// JSON-null value are represented as `None` when using `GeneratedObject`
/// accessors, so both paths collapse to `None`.  The unlocked-with-supplied-id
/// branch still guards against empty string (`""`) per the TS `!== ""` check.
use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::errors::StorageError;
use crate::generated::artifacts::models::LeaseAccessConditions;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{ILease, ILeaseValidator, LeaseStatusType};

pub struct ContainerDeleteLeaseValidator {
    pub leaseAccessConditions: Option<LeaseAccessConditions>,
}

impl ContainerDeleteLeaseValidator {
    pub fn new(lease_access_conditions: Option<LeaseAccessConditions>) -> Self {
        Self {
            leaseAccessConditions: lease_access_conditions,
        }
    }
}

impl ILeaseValidator for ContainerDeleteLeaseValidator {
    fn validate(&self, lease: &ILease, context: &Context) -> Result<(), StorageError> {
        // Helper: get leaseId string from conditions, treating null/missing as None.
        let input_lease_id_raw = self
            .leaseAccessConditions
            .as_ref()
            .and_then(|c| c.get("leaseId"))
            .and_then(|v| v.as_string()); // None for absent, null, or non-string

        if lease.leaseStatus.as_deref() == Some(LeaseStatusType::Locked) {
            // When locked, leaseId is required (null/undefined/missing all fail)
            if input_lease_id_raw.is_none() {
                return Err(StorageErrorFactory::getContainerLeaseIdMissing(
                    context.contextId().as_deref(),
                ));
            }
            let input_id = input_lease_id_raw.unwrap();
            if let Some(stored_id) = &lease.leaseId {
                if input_id.to_lowercase() != stored_id.to_lowercase() {
                    return Err(
                        StorageErrorFactory::getContainerLeaseIdMismatchWithContainerOperation(
                            context.contextId().as_deref(),
                        ),
                    );
                }
            }
        } else {
            // Unlocked: if a non-null, non-empty leaseId was supplied it is stale
            if let Some(input_id) = &input_lease_id_raw {
                if !input_id.is_empty() {
                    return Err(StorageErrorFactory::getContainerLeaseLost(
                        context.contextId().as_deref(),
                    ));
                }
            }
        }

        Ok(())
    }
}
