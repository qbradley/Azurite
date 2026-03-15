use base64::{engine::general_purpose, Engine as _};

use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{
    QueryComparison, QueryStaticType, QueryValue,
};
use crate::persistence::query_interpreter::value_node::ValueNode;

pub struct BinaryDataNode {
    inner: ValueNode,
}

impl BinaryDataNode {
    pub fn new(value: String) -> Self {
        Self {
            inner: ValueNode::new("binary", value, QueryStaticType::Binary),
        }
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(value.len() / 2);
        let mut chars = value.chars();
        while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
            let Some(high) = high.to_digit(16) else {
                return Vec::new();
            };
            let Some(low) = low.to_digit(16) else {
                return Vec::new();
            };
            bytes.push(((high << 4) | low) as u8);
        }
        bytes
    }
}

impl IQueryNode for BinaryDataNode {
    fn name(&self) -> &'static str {
        self.inner.name
    }

    fn evaluate(&self, _context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        Ok(QueryValue::String(
            general_purpose::STANDARD.encode(Self::decode_hex(&self.inner.value)),
        ))
    }

    fn compare(
        &self,
        context: &dyn IQueryContext,
        other: &dyn IQueryNode,
    ) -> Result<Option<QueryComparison>, QueryError> {
        // Must use our own evaluate() (hex→base64) rather than ValueNode::evaluate() (raw hex)
        let this_value = self.evaluate(context)?;
        let other_value = other.evaluate(context)?;
        if this_value.is_undefined() || other_value.is_undefined() || other_value.is_null() {
            return Ok(Some(QueryComparison::Nan));
        }
        Ok(Some(QueryComparison::from_ordering(
            this_value.relational_compare(&other_value),
        )))
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        Some(self.inner.static_type)
    }
}
