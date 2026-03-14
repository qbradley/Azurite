use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::generated::artifacts::mappers::Mapper;

#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ParameterPath {
    Single(String),
    Many(Vec<String>),
    Map(BTreeMap<String, ParameterPath>),
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperationParameter {
    #[serde(default)]
    pub __name: Option<String>,
    pub parameterPath: ParameterPath,
    #[serde(default)]
    pub collectionFormat: Option<String>,
    pub mapper: Mapper,
}

static PARAMETERS: LazyLock<BTreeMap<String, OperationParameter>> = LazyLock::new(BTreeMap::new);

pub fn parameters() -> &'static BTreeMap<String, OperationParameter> {
    &PARAMETERS
}

pub fn get_parameter(name: &str) -> Option<&'static OperationParameter> {
    PARAMETERS.get(name)
}
