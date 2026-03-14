/// Mirrors TypeScript `src/blob/lease/BlobLeaseSyncer.ts`.
use crate::generated::artifacts::models::GeneratedValue;
use crate::lease::i_lease_state::{ILease, ILeaseSyncer};
use crate::persistence::BlobModel;

pub struct BlobLeaseSyncer<'a> {
    pub blob: &'a mut BlobModel,
}

impl<'a> BlobLeaseSyncer<'a> {
    pub fn new(blob: &'a mut BlobModel) -> Self {
        Self { blob }
    }
}

impl<'a> ILeaseSyncer<BlobModel> for BlobLeaseSyncer<'a> {
    fn sync(&mut self, lease: &ILease) -> BlobModel {
        self.blob.leaseId = lease.leaseId.clone();
        self.blob.leaseExpireTime = lease.leaseExpireTime;
        self.blob.leaseDurationSeconds = lease.leaseDurationSeconds;
        self.blob.leaseBreakTime = lease.leaseBreakTime;

        if let Some(v) = &lease.leaseDurationType {
            self.blob.properties.insert(
                "leaseDuration".to_string(),
                GeneratedValue::String(v.clone()),
            );
        } else {
            self.blob.properties.remove("leaseDuration");
        }

        if let Some(v) = &lease.leaseState {
            self.blob
                .properties
                .insert("leaseState".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.blob.properties.remove("leaseState");
        }

        if let Some(v) = &lease.leaseStatus {
            self.blob
                .properties
                .insert("leaseStatus".to_string(), GeneratedValue::String(v.clone()));
        } else {
            self.blob.properties.remove("leaseStatus");
        }

        self.blob.clone()
    }
}
