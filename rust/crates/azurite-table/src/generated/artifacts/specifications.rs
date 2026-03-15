use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::generated::artifacts::mappers::Mapper;
use crate::generated::artifacts::operation::Operation;
use crate::generated::artifacts::parameters::{OperationParameter, ParameterPath};

#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestBodySpec {
    pub parameterPath: ParameterPath,
    pub mapper: Mapper,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ResponseSpec {
    #[serde(default)]
    pub bodyMapper: Option<Mapper>,
    #[serde(default)]
    pub headersMapper: Option<Mapper>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperationSpec {
    pub operation: String,
    pub httpMethod: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub urlParameters: Vec<OperationParameter>,
    #[serde(default)]
    pub queryParameters: Vec<OperationParameter>,
    #[serde(default)]
    pub headerParameters: Vec<OperationParameter>,
    #[serde(default)]
    pub requestBody: Option<RequestBodySpec>,
    #[serde(default)]
    pub contentType: Option<String>,
    #[serde(default)]
    pub responses: BTreeMap<String, ResponseSpec>,
    #[serde(default)]
    pub isXML: bool,
}

static SPECIFICATIONS: LazyLock<Vec<OperationSpec>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("metadata/specifications.generated.json"))
        .expect("generated table specification metadata must deserialize")
});

pub fn specifications() -> &'static [OperationSpec] {
    SPECIFICATIONS.as_slice()
}

pub fn specification(operation: Operation) -> Option<&'static OperationSpec> {
    SPECIFICATIONS.get(operation.as_usize())
}
