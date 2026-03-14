/// Mirrors TypeScript `src/blob/lease/LeaseStateBase.ts`.
use crate::errors::StorageError;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{ILease, ILeaseValidator};

/// Common base for all lease states.
/// TypeScript uses class inheritance; Rust uses composition — embed this as a field.
#[derive(Debug, Clone)]
pub struct LeaseStateBase {
    pub lease: ILease,
    pub context: Context,
}

impl LeaseStateBase {
    pub fn new(lease: ILease, context: Context) -> Self {
        Self { lease, context }
    }

    /// Mirrors `LeaseStateBase.validate()`: runs the validator against the stored lease
    /// and context.  Returns `Ok(())` on success; the concrete state impl returns `Ok(self)`
    /// after calling this.
    pub fn validate(&self, validator: &dyn ILeaseValidator) -> Result<(), StorageError> {
        validator.validate(&self.lease, &self.context)
    }
}
