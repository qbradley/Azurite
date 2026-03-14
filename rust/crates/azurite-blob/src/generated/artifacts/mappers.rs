use std::collections::BTreeMap;
use std::sync::LazyLock;

#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct MapperType {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub className: Option<String>,
    #[serde(default)]
    pub allowedValues: Vec<String>,
    #[serde(default)]
    pub element: Option<Box<Mapper>>,
    #[serde(default)]
    pub value: Option<Box<Mapper>>,
    #[serde(default)]
    pub modelProperties: BTreeMap<String, Mapper>,
    #[serde(default)]
    pub additionalProperties: Option<Box<Mapper>>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Mapper {
    #[serde(default)]
    pub __name: Option<String>,
    #[serde(default)]
    pub serializedName: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub isConstant: bool,
    #[serde(default)]
    pub defaultValue: Option<serde_json::Value>,
    #[serde(default)]
    pub xmlName: Option<String>,
    #[serde(default)]
    pub xmlElementName: Option<String>,
    #[serde(default)]
    pub xmlIsWrapped: bool,
    #[serde(default)]
    pub headerCollectionPrefix: Option<String>,
    #[serde(default, rename = "type")]
    pub r#type: MapperType,
}

static MAPPERS: LazyLock<BTreeMap<String, Mapper>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("metadata/mappers.generated.json"))
        .expect("generated blob mapper metadata must deserialize")
});

pub fn mappers() -> &'static BTreeMap<String, Mapper> {
    &MAPPERS
}

pub fn get_mapper(name: &str) -> Option<&'static Mapper> {
    MAPPERS.get(name)
}
