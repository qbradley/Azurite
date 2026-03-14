/// Mirrors TypeScript `src/blob/lease/LeaseBreakingState.ts`.
///
/// Fidelity note: `change()` only checks the *first* argument (`lease_id`) against the
/// stored lease ID, matching the TypeScript signature mismatch where the implementation
/// only declares one parameter (`proposedLeaseId`) which therefore receives position-0.
use azurite_common::utils::utils::minDate;

use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{ILease, ILeaseState, ILeaseValidator, LeaseStateType, LeaseStatusType};
use crate::lease::lease_state_base::LeaseStateBase;

pub struct LeaseBreakingState {
    pub base: LeaseStateBase,
}

impl LeaseBreakingState {
    pub fn new(lease: ILease, context: Context) -> Result<Self, String> {
        if context.startTime().is_none() {
            return Err(
                "LeaseLeasedState:constructor() error, context.startTime is undefined.".into(),
            );
        }
        if lease.leaseState.as_deref() != Some(LeaseStateType::Breaking) {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming lease state {:?} is not {}.",
                lease.leaseState,
                LeaseStateType::Breaking
            ));
        }
        if lease.leaseStatus.as_deref() != Some(LeaseStatusType::Locked) {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming lease status {:?} is not {}.",
                lease.leaseStatus,
                LeaseStatusType::Locked
            ));
        }
        if lease.leaseId.is_none() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseId {:?} should not be undefined.",
                lease.leaseId
            ));
        }
        if lease.leaseExpireTime.is_some() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseExpireTime {:?} is not undefined.",
                lease.leaseExpireTime
            ));
        }
        if lease.leaseDurationSeconds.is_some() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseDurationSeconds {:?} is not undefined.",
                lease.leaseDurationSeconds
            ));
        }
        if lease.leaseDurationType.is_some() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseDurationType {:?} is not undefined.",
                lease.leaseDurationType
            ));
        }
        let break_time = lease.leaseBreakTime;
        let start = context.startTime().unwrap();
        if break_time.is_none() || start >= break_time.unwrap() {
            return Err(format!(
                "LeaseLeasedState:constructor() error, incoming leaseBreakTime {:?} is undefined, or less than current time {:?}.",
                break_time, start
            ));
        }

        Ok(Self {
            base: LeaseStateBase::new(lease, context),
        })
    }
}

impl ILeaseState for LeaseBreakingState {
    fn lease(&self) -> &ILease {
        &self.base.lease
    }

    fn acquire(
        &self,
        _duration: i64,
        proposed_lease_id: Option<&str>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if proposed_lease_id == self.base.lease.leaseId.as_deref() {
            Err(StorageErrorFactory::getLeaseIsBreakingAndCannotBeAcquired(
                self.base.context.contextId().as_deref(),
            ))
        } else {
            Err(StorageErrorFactory::getLeaseAlreadyPresent(
                self.base.context.contextId().as_deref(),
            ))
        }
    }

    fn break_lease(
        &self,
        break_period: Option<i64>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if break_period.is_none() {
            // Return self unchanged — clone current breaking state
            return Ok(Box::new(
                LeaseBreakingState::new(self.base.lease.clone(), self.base.context.clone())
                    .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
            ));
        }

        let bp = break_period.unwrap();

        if bp == 0 {
            return Ok(Box::new(
                crate::lease::lease_broken_state::LeaseBrokenState::new(
                    ILease {
                        leaseId: self.base.lease.leaseId.clone(),
                        leaseState: Some(LeaseStateType::Broken.to_string()),
                        leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                        leaseDurationType: None,
                        leaseDurationSeconds: None,
                        leaseExpireTime: None,
                        leaseBreakTime: None,
                    },
                    self.base.context.clone(),
                )
                .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
            ));
        }

        if bp > 0 && bp <= 60 {
            let start = self.base.context.startTime().unwrap();
            let break_time = start + chrono::Duration::seconds(bp);
            let actual = minDate(self.base.lease.leaseBreakTime.unwrap(), break_time);
            return Ok(Box::new(
                LeaseBreakingState::new(
                    ILease {
                        leaseId: self.base.lease.leaseId.clone(),
                        leaseState: Some(LeaseStateType::Breaking.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: None,
                        leaseDurationSeconds: None,
                        leaseExpireTime: None,
                        leaseBreakTime: Some(actual),
                    },
                    self.base.context.clone(),
                )
                .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
            ));
        }

        Err(StorageErrorFactory::getInvalidLeaseBreakPeriod(
            self.base.context.contextId().as_deref(),
        ))
    }

    fn renew(&self, proposed_lease_id: &str) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if proposed_lease_id == self.base.lease.leaseId.as_deref().unwrap_or("") {
            Err(StorageErrorFactory::getLeaseIsBrokenAndCannotBeRenewed(
                self.base.context.contextId().as_deref(),
            ))
        } else {
            Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
                self.base.context.contextId().as_deref(),
            ))
        }
    }

    /// Fidelity: the TS implementation only checks the *first* argument against
    /// `this.lease.leaseId` (the `proposedLeaseId` local param receives position-0).
    fn change(
        &self,
        lease_id: &str,
        _proposed_lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if lease_id == self.base.lease.leaseId.as_deref().unwrap_or("") {
            Err(StorageErrorFactory::getLeaseIsBreakingAndCannotBeChanged(
                self.base.context.contextId().as_deref(),
            ))
        } else {
            Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
                self.base.context.contextId().as_deref(),
            ))
        }
    }

    fn release(
        &self,
        lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if self.base.lease.leaseId.as_deref() != Some(lease_id) {
            return Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
                self.base.context.contextId().as_deref(),
            ));
        }

        Ok(Box::new(
            crate::lease::lease_available_state::LeaseAvailableState::new(
                ILease {
                    leaseId: None,
                    leaseState: Some(LeaseStateType::Available.to_string()),
                    leaseStatus: Some(LeaseStatusType::Unlocked.to_string()),
                    leaseDurationType: None,
                    leaseDurationSeconds: None,
                    leaseExpireTime: None,
                    leaseBreakTime: None,
                },
                self.base.context.clone(),
            )
            .map_err(|e| crate::errors::StorageError::new(500, "InternalError", &e, "", std::collections::BTreeMap::new()))?,
        ))
    }

    fn validate(&self, validator: &dyn ILeaseValidator) -> Result<(), crate::errors::StorageError> {
        self.base.validate(validator)
    }
}
