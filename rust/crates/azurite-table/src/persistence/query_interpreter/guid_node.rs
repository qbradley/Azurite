use std::sync::OnceLock;

use base64::{engine::general_purpose, Engine as _};
use regex::Regex;

use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{
    QueryComparison, QueryStaticType, QueryValue,
};
use crate::persistence::query_interpreter::value_node::ValueNode;

pub struct GuidNode {
    inner: ValueNode,
}

impl GuidNode {
    pub fn new(value: String) -> Self {
        Self {
            inner: ValueNode::new("guid", value, QueryStaticType::Guid),
        }
    }

    fn guid_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new(r"^[0-9a-f]{8}-([0-9a-f]{4}-){3}[0-9a-f]{12}$").unwrap())
    }
}

impl IQueryNode for GuidNode {
    fn name(&self) -> &'static str {
        self.inner.name
    }

    fn evaluate(&self, _context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        Ok(QueryValue::String(
            general_purpose::STANDARD.encode(self.inner.value.as_bytes()),
        ))
    }

    fn compare(
        &self,
        context: &dyn IQueryContext,
        other: &dyn IQueryNode,
    ) -> Result<Option<QueryComparison>, QueryError> {
        let other_value = other.evaluate(context)?.to_string_value();
        let Some(other_value) = other_value else {
            return Ok(Some(QueryComparison::Nan));
        };

        let this_value = if Self::guid_regex().is_match(&other_value) {
            self.inner.value.clone()
        } else {
            general_purpose::STANDARD.encode(self.inner.value.as_bytes())
        };

        if this_value.is_empty() || other_value.is_empty() {
            return Ok(Some(QueryComparison::Nan));
        }

        Ok(Some(QueryComparison::from_ordering(Some(
            this_value.cmp(&other_value),
        ))))
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        Some(self.inner.static_type)
    }
}
