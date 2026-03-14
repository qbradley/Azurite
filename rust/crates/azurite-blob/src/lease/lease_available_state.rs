/// Mirrors TypeScript `src/blob/lease/LeaseAvailableState.ts`.
use uuid::Uuid;

use crate::errors::storage_error_factory::StorageErrorFactory;
use crate::generated::context::Context;
use crate::lease::i_lease_state::{
    ILease, ILeaseState, ILeaseValidator, LeaseDurationType, LeaseStateType, LeaseStatusType,
};
use crate::lease::lease_state_base::LeaseStateBase;

pub struct LeaseAvailableState {
    pub base: LeaseStateBase,
}

impl LeaseAvailableState {
    pub fn new(lease: ILease, context: Context) -> Result<Self, String> {
        if context.startTime().is_none() {
            return Err(
                "LeaseAvailableState:constructor() error, context.startTime is undefined.".into(),
            );
        }

        if lease.leaseState.is_none() {
            // Silently normalise undefined → available (mirrors TS commented-out throw path)
            return Ok(Self {
                base: LeaseStateBase::new(
                    ILease {
                        leaseId: None,
                        leaseState: None,
                        leaseStatus: None,
                        leaseDurationType: None,
                        leaseDurationSeconds: None,
                        leaseExpireTime: None,
                        leaseBreakTime: None,
                    },
                    context,
                ),
            });
        }

        if lease.leaseState.as_deref() != Some(LeaseStateType::Available) {
            return Err(format!(
                "LeaseAvailableState:constructor() error, incoming lease state {:?} is not {}.",
                lease.leaseState,
                LeaseStateType::Available
            ));
        }
        if lease.leaseStatus.as_deref() != Some(LeaseStatusType::Unlocked) {
            return Err(format!(
                "LeaseAvailableState:constructor() error, incoming lease status {:?} is not {}.",
                lease.leaseStatus,
                LeaseStatusType::Unlocked
            ));
        }
        if lease.leaseId.is_some() {
            return Err(format!(
                "LeaseAvailableState:constructor() error, incoming leaseId {:?} is not undefined.",
                lease.leaseId
            ));
        }
        if lease.leaseExpireTime.is_some() {
            return Err(format!(
                "LeaseAvailableState:constructor() error, incoming leaseExpireTime {:?} is not undefined.",
                lease.leaseExpireTime
            ));
        }
        if lease.leaseDurationSeconds.is_some() {
            return Err(format!(
                "LeaseAvailableState:constructor() error, incoming leaseDurationSeconds {:?} is not undefined.",
                lease.leaseDurationSeconds
            ));
        }
        if lease.leaseDurationType.is_some() {
            return Err(format!(
                "LeaseAvailableState:constructor() error, incoming leaseDurationType {:?} is not undefined.",
                lease.leaseDurationType
            ));
        }
        if lease.leaseBreakTime.is_some() {
            return Err(format!(
                "LeaseAvailableState:constructor() error, incoming leaseBreakTime {:?} is not undefined.",
                lease.leaseBreakTime
            ));
        }

        // Deep copy (clone)
        Ok(Self {
            base: LeaseStateBase::new(lease, context),
        })
    }
}

impl ILeaseState for LeaseAvailableState {
    fn lease(&self) -> &ILease {
        &self.base.lease
    }

    fn acquire(
        &self,
        duration: i64,
        proposed_lease_id: Option<&str>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        if !(15..=60).contains(&duration) && duration != -1 {
            return Err(StorageErrorFactory::getInvalidLeaseDuration(
                self.base.context.contextId().as_deref(),
            ));
        }

        let lease_id = proposed_lease_id
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let start = self.base.context.startTime().unwrap();

        if duration == -1 {
            Ok(Box::new(
                crate::lease::lease_leased_state::LeaseLeasedState::new(
                    ILease {
                        leaseId: Some(lease_id),
                        leaseState: Some(LeaseStateType::Leased.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: Some(LeaseDurationType::Infinite.to_string()),
                        leaseDurationSeconds: None,
                        leaseExpireTime: None,
                        leaseBreakTime: None,
                    },
                    self.base.context.clone(),
                )
                .map_err(|e| {
                    crate::errors::StorageError::new(
                        500,
                        "InternalError",
                        &e,
                        "",
                        std::collections::BTreeMap::new(),
                    )
                })?,
            ))
        } else {
            let expire = start + chrono::Duration::seconds(duration);
            Ok(Box::new(
                crate::lease::lease_leased_state::LeaseLeasedState::new(
                    ILease {
                        leaseId: Some(lease_id),
                        leaseState: Some(LeaseStateType::Leased.to_string()),
                        leaseStatus: Some(LeaseStatusType::Locked.to_string()),
                        leaseDurationType: Some(LeaseDurationType::Fixed.to_string()),
                        leaseDurationSeconds: Some(duration),
                        leaseExpireTime: Some(expire),
                        leaseBreakTime: None,
                    },
                    self.base.context.clone(),
                )
                .map_err(|e| {
                    crate::errors::StorageError::new(
                        500,
                        "InternalError",
                        &e,
                        "",
                        std::collections::BTreeMap::new(),
                    )
                })?,
            ))
        }
    }

    fn break_lease(
        &self,
        _break_period: Option<i64>,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Err(StorageErrorFactory::getLeaseNotPresentWithLeaseOperation(
            self.base.context.contextId().as_deref(),
        ))
    }

    fn renew(&self, _lease_id: &str) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
            self.base.context.contextId().as_deref(),
        ))
    }

    fn change(
        &self,
        _lease_id: &str,
        _proposed_lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Err(StorageErrorFactory::getLeaseNotPresentWithLeaseOperation(
            self.base.context.contextId().as_deref(),
        ))
    }

    fn release(
        &self,
        _lease_id: &str,
    ) -> Result<Box<dyn ILeaseState>, crate::errors::StorageError> {
        Err(StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(
            self.base.context.contextId().as_deref(),
        ))
    }

    fn validate(&self, validator: &dyn ILeaseValidator) -> Result<(), crate::errors::StorageError> {
        self.base.validate(validator)
    }
}
