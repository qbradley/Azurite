use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{QueryStaticType, QueryValue};

pub struct ConstantNode {
    value: QueryValue,
    static_type: QueryStaticType,
}

impl ConstantNode {
    pub fn boolean(value: bool) -> Self {
        Self {
            value: QueryValue::Bool(value),
            static_type: QueryStaticType::Boolean,
        }
    }

    pub fn number(value: f64) -> Self {
        Self {
            value: QueryValue::Number(value),
            static_type: QueryStaticType::Number,
        }
    }

    pub fn string(value: String) -> Self {
        Self {
            value: QueryValue::String(value),
            static_type: QueryStaticType::String,
        }
    }
}

impl IQueryNode for ConstantNode {
    fn name(&self) -> &'static str {
        "constant"
    }

    fn evaluate(&self, _context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        Ok(self.value.clone())
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        Some(self.static_type)
    }
}
