use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{
    QueryComparison, QueryStaticType, QueryValue,
};
use crate::persistence::query_interpreter::value_node::ValueNode;

pub struct BigNumberNode {
    inner: ValueNode,
}

impl BigNumberNode {
    pub fn new(value: String) -> Self {
        Self {
            inner: ValueNode::new("BigNumber", value, QueryStaticType::Long),
        }
    }

    fn compare_positive_number(this_value: &str, other_value: &str) -> QueryComparison {
        let this_number_value = Self::trim_zeros(this_value);
        let other_number_value = Self::trim_zeros(other_value);

        if this_number_value.len() < other_number_value.len() {
            return QueryComparison::Less;
        }
        if this_number_value.len() > other_number_value.len() {
            return QueryComparison::Greater;
        }

        for (left, right) in this_number_value.chars().zip(other_number_value.chars()) {
            if left < right {
                return QueryComparison::Less;
            }
            if left > right {
                return QueryComparison::Greater;
            }
        }

        QueryComparison::Equal
    }

    fn trim_zeros(number_string: &str) -> &str {
        number_string.trim_start_matches('0')
    }
}

impl IQueryNode for BigNumberNode {
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
        let this_value = self.inner.value.as_str();
        let other_value = other.evaluate(context)?.to_string_value();
        let Some(other_value) = other_value else {
            return Ok(Some(QueryComparison::Nan));
        };

        if other_value == "null" {
            return Ok(Some(QueryComparison::Nan));
        }

        let result = if let Some(stripped_this) = this_value.strip_prefix('-') {
            if let Some(stripped_other) = other_value.strip_prefix('-') {
                match Self::compare_positive_number(stripped_this, stripped_other) {
                    QueryComparison::Less => QueryComparison::Greater,
                    QueryComparison::Equal => QueryComparison::Equal,
                    QueryComparison::Greater => QueryComparison::Less,
                    QueryComparison::Nan => QueryComparison::Nan,
                }
            } else if Self::trim_zeros(stripped_this).is_empty()
                && Self::trim_zeros(&other_value).is_empty()
            {
                QueryComparison::Equal
            } else {
                QueryComparison::Less
            }
        } else if let Some(stripped_other) = other_value.strip_prefix('-') {
            if Self::trim_zeros(this_value).is_empty()
                && Self::trim_zeros(stripped_other).is_empty()
            {
                QueryComparison::Equal
            } else {
                QueryComparison::Greater
            }
        } else {
            Self::compare_positive_number(this_value, &other_value)
        };

        Ok(Some(result))
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        Some(self.inner.static_type)
    }
}
