use std::collections::HashMap;

/// Mirrors TypeScript `type IQueryContext = any`.
///
/// Actual callers treat the context as `tagName -> value` plus `@container`.
/// Keep runtime key lookup visible rather than over-constraining the type.
pub type IQueryContext = HashMap<String, String>;
