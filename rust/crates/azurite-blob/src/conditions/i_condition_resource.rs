use chrono::{DateTime, Utc};

use crate::persistence::FilterBlobModel;

/// Mirrors TypeScript `IConditionResource` interface.
#[allow(non_snake_case)]
#[derive(Clone, Debug)]
pub struct IConditionResource {
    /// Whether resource exists or not.
    pub exist: bool,

    /// etag string without quotes.
    pub etag: String,

    /// last modified time for container or blob.
    pub lastModified: Option<DateTime<Utc>>,

    pub blobItemWithTags: Option<FilterBlobModel>,
}

impl Default for IConditionResource {
    fn default() -> Self {
        Self {
            exist: false,
            etag: String::new(),
            lastModified: None,
            blobItemWithTags: None,
        }
    }
}
