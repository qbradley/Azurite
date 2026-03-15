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
    let mut specs: Vec<OperationSpec> =
        serde_json::from_str(include_str!("metadata/specifications.generated.json"))
            .expect("generated table specification metadata must deserialize");

    // Runtime overrides matching TS TableRequestListenerFactory.ts:
    // Many table entity operations use JSON, not XML, despite spec saying isXML=true
    for spec in &mut specs {
        let op = spec.operation.as_str();
        match op {
            "Table_Create"
            | "Table_Query"
            | "Table_Delete"
            | "Table_QueryEntities"
            | "Table_QueryEntitiesWithPartitionAndRowKey"
            | "Table_UpdateEntity"
            | "Table_MergeEntity"
            | "Table_DeleteEntity"
            | "Table_InsertEntity" => {
                spec.isXML = false;
            }
            // MERGE verb not supported by autorest generator, so the spec uses POST
            // but Azure storage SDK sends MERGE HTTP method
            "Table_MergeEntityWithMerge" => {
                spec.httpMethod = String::from("MERGE");
                spec.isXML = false;
            }
            _ => {}
        }
    }

    specs
});

pub fn specifications() -> &'static [OperationSpec] {
    SPECIFICATIONS.as_slice()
}

pub fn specification(operation: Operation) -> Option<&'static OperationSpec> {
    SPECIFICATIONS.get(operation.as_usize())
}
