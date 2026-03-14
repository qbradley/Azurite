/// Mirrors TypeScript `src/blob/lease/LeaseFactory.ts`.
///
/// Fidelity note: This is a lazy factory — no background timer is introduced.
/// The factory inspects the stored `leaseState`, `leaseExpireTime`, and
/// `leaseBreakTime` fields against `context.startTime` to determine which
/// concrete state to instantiate.
use crate::errors::StorageError;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{ILease, ILeaseState, LeaseStateType};
use crate::lease::lease_available_state::LeaseAvailableState;
use crate::lease::lease_breaking_state::LeaseBreakingState;
use crate::lease::lease_broken_state::LeaseBrokenState;
use crate::lease::lease_expired_state::LeaseExpiredState;
use crate::lease::lease_leased_state::LeaseLeasedState;

pub struct LeaseFactory;

impl LeaseFactory {
    pub fn create_lease_state(
        lease: &ILease,
        context: &Context,
    ) -> Result<Box<dyn ILeaseState>, StorageError> {
        Self::createLeaseState(lease.clone(), context.clone())
    }

    pub fn createLeaseState(
        lease: ILease,
        context: Context,
    ) -> Result<Box<dyn ILeaseState>, StorageError> {
        if context.startTime().is_none() {
            return Err(StorageError::new(
                500,
                "InternalError",
                "LeaseFactory:createLeaseState() context.startTime should not be undefined.",
                "",
                std::collections::BTreeMap::new(),
            ));
        }

        let start = context.startTime().unwrap();

        match lease.leaseState.as_deref() {
            Some(s) if s == LeaseStateType::Available => {
                LeaseAvailableState::new(lease, context)
                    .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                    .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
            }
            None => {
                LeaseAvailableState::new(lease, context)
                    .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                    .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
            }
            Some(s) if s == LeaseStateType::Leased => {
                // Check if the fixed lease has expired
                if lease.leaseExpireTime.is_none() || start < lease.leaseExpireTime.unwrap() {
                    LeaseLeasedState::new(lease, context)
                        .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                        .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
                } else {
                    LeaseExpiredState::new(lease, context)
                        .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                        .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
                }
            }
            Some(s) if s == LeaseStateType::Expired => {
                LeaseExpiredState::new(lease, context)
                    .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                    .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
            }
            Some(s) if s == LeaseStateType::Breaking => {
                if lease.leaseBreakTime.is_none() {
                    return Err(StorageError::new(
                        500,
                        "InternalError",
                        format!(
                            "LeaseFactory:createLeaseState() leaseBreakTime should not be undefined when leaseState is {}.",
                            LeaseStateType::Breaking
                        ),
                        "",
                        std::collections::BTreeMap::new(),
                    ));
                }
                if start < lease.leaseBreakTime.unwrap() {
                    LeaseBreakingState::new(lease, context)
                        .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                        .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
                } else {
                    LeaseBrokenState::new(lease, context)
                        .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                        .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
                }
            }
            Some(s) if s == LeaseStateType::Broken => {
                LeaseBrokenState::new(lease, context)
                    .map(|s| Box::new(s) as Box<dyn ILeaseState>)
                    .map_err(|e| StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))
            }
            Some(other) => Err(StorageError::new(
                500,
                "InternalError",
                format!(
                    "LeaseFactory:createLeaseState() Cannot create LeaseState instance from lease with state {:?}",
                    other
                ),
                "",
                std::collections::BTreeMap::new(),
            )),
        }
    }
}
