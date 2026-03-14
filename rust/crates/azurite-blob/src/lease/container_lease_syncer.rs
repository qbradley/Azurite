/// Mirrors TypeScript `src/blob/lease/ContainerLeaseSyncer.ts`.
use crate::generated::artifacts::models::GeneratedValue;
use crate::lease::i_lease_state::{ILease, ILeaseSyncer};
use crate::persistence::ContainerModel;

pub struct ContainerLeaseSyncer<'a> {
    pub container: &'a mut ContainerModel,
}

impl<'a> ContainerLeaseSyncer<'a> {
    pub fn new(container: &'a mut ContainerModel) -> Self {
        Self { container }
    }
}

impl<'a> ILeaseSyncer<ContainerModel> for ContainerLeaseSyncer<'a> {
    fn sync(&mut self, lease: &ILease) -> ContainerModel {
        self.container.leaseId = lease.leaseId.clone();
        self.container.leaseExpireTime = lease.leaseExpireTime;
        self.container.leaseDurationSeconds = lease.leaseDurationSeconds;
        self.container.leaseBreakTime = lease.leaseBreakTime;

        if let Some(v) = &lease.leaseDurationType {
            self.container
                .properties
                .insert("leaseDuration".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.container.properties.remove("leaseDuration");
        }

        if let Some(v) = &lease.leaseState {
            self.container
                .properties
                .insert("leaseState".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.container.properties.remove("leaseState");
        }

        if let Some(v) = &lease.leaseStatus {
            self.container
                .properties
                .insert("leaseStatus".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.container.properties.remove("leaseStatus");
        }

        self.container.clone()
    }
}
