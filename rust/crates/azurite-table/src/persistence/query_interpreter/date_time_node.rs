use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{
    QueryComparison, QueryStaticType, QueryValue,
};
use crate::persistence::query_interpreter::value_node::ValueNode;

pub struct DateTimeNode {
    inner: ValueNode,
}

impl DateTimeNode {
    pub fn new(value: String) -> Self {
        Self {
            inner: ValueNode::new("datetime", value, QueryStaticType::DateTime),
        }
    }

    fn parse_date_string(value: &str) -> Option<i64> {
        if let Ok(value) = DateTime::parse_from_rfc3339(value) {
            return Some(value.timestamp_millis());
        }

        if let Ok(value) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f") {
            return Some(DateTime::<Utc>::from_naive_utc_and_offset(value, Utc).timestamp_millis());
        }

        if let Ok(value) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            return value.and_hms_opt(0, 0, 0).map(|value| {
                DateTime::<Utc>::from_naive_utc_and_offset(value, Utc).timestamp_millis()
            });
        }

        None
    }

    fn parse_query_value(value: &QueryValue) -> Option<i64> {
        match value {
            QueryValue::Undefined | QueryValue::Null => None,
            QueryValue::Bool(value) => Some(if *value { 1 } else { 0 }),
            QueryValue::Number(value) if value.is_finite() => Some(*value as i64),
            QueryValue::Number(_) => None,
            QueryValue::String(value) => Self::parse_date_string(value),
        }
    }
}

impl IQueryNode for DateTimeNode {
    fn name(&self) -> &'static str {
        self.inner.name
    }

    fn evaluate(&self, _context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        Ok(self.inner.evaluate())
    }

    fn compare(
        &self,
        context: &dyn IQueryContext,
        other: &dyn IQueryNode,
    ) -> Result<Option<QueryComparison>, QueryError> {
        let other_value = other.evaluate(context)?;
        if other_value.is_null() {
            return Ok(Some(QueryComparison::Nan));
        }

        let Some(this_date) = Self::parse_date_string(&self.inner.value) else {
            return Ok(Some(QueryComparison::Nan));
        };
        let Some(other_date) = Self::parse_query_value(&other_value) else {
            return Ok(Some(QueryComparison::Nan));
        };

        Ok(Some(QueryComparison::from_ordering(
            this_date.partial_cmp(&other_date),
        )))
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        Some(self.inner.static_type)
    }
}
