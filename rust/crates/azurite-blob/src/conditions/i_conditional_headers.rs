use chrono::{DateTime, Utc};

/// Mirrors TypeScript `IConditionalHeaders` interface.
///
/// Conditional header values for blob service operations.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct IConditionalHeaders {
    pub ifModifiedSince: Option<DateTime<Utc>>,
    pub ifUnmodifiedSince: Option<DateTime<Utc>>,

    /// If-Match etag list without quotes.
    pub ifMatch: Option<Vec<String>>,

    /// If-None-Match etag list without quotes.
    pub ifNoneMatch: Option<Vec<String>>,

    /// Specify a SQL where clause on blob tags to operate only on blobs with a matching value.
    pub ifTags: Option<String>,
}
