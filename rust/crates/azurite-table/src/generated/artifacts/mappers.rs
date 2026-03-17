use std::collections::BTreeMap;
use std::sync::LazyLock;

use indexmap::IndexMap;

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
    pub modelProperties: IndexMap<String, Mapper>,
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
    BTreeMap::from([
        (
            String::from("RetentionPolicy"),
            composite_mapper(
                "RetentionPolicy",
                "RetentionPolicy",
                IndexMap::from([
                    (
                        String::from("enabled"),
                        boolean_mapper("Enabled", "Enabled", true),
                    ),
                    (String::from("days"), number_mapper("Days", "Days", false)),
                ]),
            ),
        ),
        (
            String::from("Logging"),
            composite_mapper(
                "Logging",
                "Logging",
                IndexMap::from([
                    (
                        String::from("version"),
                        string_mapper("Version", "Version", true),
                    ),
                    (
                        String::from("deleteProperty"),
                        boolean_mapper("Delete", "Delete", true),
                    ),
                    (String::from("read"), boolean_mapper("Read", "Read", true)),
                    (
                        String::from("write"),
                        boolean_mapper("Write", "Write", true),
                    ),
                    (
                        String::from("retentionPolicy"),
                        composite_ref_mapper(
                            "RetentionPolicy",
                            "RetentionPolicy",
                            "RetentionPolicy",
                            true,
                        ),
                    ),
                ]),
            ),
        ),
        (
            String::from("Metrics"),
            composite_mapper(
                "Metrics",
                "Metrics",
                IndexMap::from([
                    (
                        String::from("version"),
                        string_mapper("Version", "Version", false),
                    ),
                    (
                        String::from("enabled"),
                        boolean_mapper("Enabled", "Enabled", true),
                    ),
                    (
                        String::from("includeAPIs"),
                        boolean_mapper("IncludeAPIs", "IncludeAPIs", false),
                    ),
                    (
                        String::from("retentionPolicy"),
                        composite_ref_mapper(
                            "RetentionPolicy",
                            "RetentionPolicy",
                            "RetentionPolicy",
                            false,
                        ),
                    ),
                ]),
            ),
        ),
        (
            String::from("CorsRule"),
            composite_mapper(
                "CorsRule",
                "CorsRule",
                IndexMap::from([
                    (
                        String::from("allowedOrigins"),
                        string_mapper("AllowedOrigins", "AllowedOrigins", true),
                    ),
                    (
                        String::from("allowedMethods"),
                        string_mapper("AllowedMethods", "AllowedMethods", true),
                    ),
                    (
                        String::from("allowedHeaders"),
                        string_mapper("AllowedHeaders", "AllowedHeaders", true),
                    ),
                    (
                        String::from("exposedHeaders"),
                        string_mapper("ExposedHeaders", "ExposedHeaders", true),
                    ),
                    (
                        String::from("maxAgeInSeconds"),
                        number_mapper("MaxAgeInSeconds", "MaxAgeInSeconds", true),
                    ),
                ]),
            ),
        ),
        (
            String::from("GeoReplication"),
            composite_mapper(
                "GeoReplication",
                "GeoReplication",
                IndexMap::from([
                    (
                        String::from("status"),
                        string_mapper("Status", "Status", true),
                    ),
                    (
                        String::from("lastSyncTime"),
                        datetime_mapper("LastSyncTime", "LastSyncTime", true),
                    ),
                ]),
            ),
        ),
        (
            String::from("TableProperties"),
            composite_mapper(
                "TableProperties",
                "TableProperties",
                IndexMap::from([(
                    String::from("tableName"),
                    string_mapper("TableName", "TableName", false),
                )]),
            ),
        ),
        (
            String::from("AccessPolicy"),
            composite_mapper(
                "AccessPolicy",
                "AccessPolicy",
                IndexMap::from([
                    (String::from("start"), string_mapper("Start", "Start", true)),
                    (
                        String::from("expiry"),
                        string_mapper("Expiry", "Expiry", true),
                    ),
                    (
                        String::from("permission"),
                        string_mapper("Permission", "Permission", true),
                    ),
                ]),
            ),
        ),
        (
            String::from("SignedIdentifier"),
            composite_mapper(
                "SignedIdentifier",
                "SignedIdentifier",
                IndexMap::from([
                    (String::from("id"), string_mapper("Id", "Id", true)),
                    (
                        String::from("accessPolicy"),
                        composite_ref_mapper("AccessPolicy", "AccessPolicy", "AccessPolicy", true),
                    ),
                ]),
            ),
        ),
    ])
});

fn composite_mapper(
    serialized_name: &str,
    class_name: &str,
    model_properties: IndexMap<String, Mapper>,
) -> Mapper {
    Mapper {
        serializedName: Some(serialized_name.to_string()),
        r#type: MapperType {
            name: String::from("Composite"),
            className: Some(class_name.to_string()),
            modelProperties: model_properties,
            ..MapperType::default()
        },
        ..Mapper::default()
    }
}

fn string_mapper(serialized_name: &str, xml_name: &str, required: bool) -> Mapper {
    primitive_mapper(serialized_name, xml_name, required, "String")
}

fn boolean_mapper(serialized_name: &str, xml_name: &str, required: bool) -> Mapper {
    primitive_mapper(serialized_name, xml_name, required, "Boolean")
}

fn number_mapper(serialized_name: &str, xml_name: &str, required: bool) -> Mapper {
    primitive_mapper(serialized_name, xml_name, required, "Number")
}

fn datetime_mapper(serialized_name: &str, xml_name: &str, required: bool) -> Mapper {
    primitive_mapper(serialized_name, xml_name, required, "DateTimeRfc1123")
}

fn primitive_mapper(
    serialized_name: &str,
    xml_name: &str,
    required: bool,
    type_name: &str,
) -> Mapper {
    Mapper {
        serializedName: Some(serialized_name.to_string()),
        xmlName: Some(xml_name.to_string()),
        required,
        r#type: MapperType {
            name: type_name.to_string(),
            ..MapperType::default()
        },
        ..Mapper::default()
    }
}

fn composite_ref_mapper(
    serialized_name: &str,
    xml_name: &str,
    class_name: &str,
    required: bool,
) -> Mapper {
    Mapper {
        serializedName: Some(serialized_name.to_string()),
        xmlName: Some(xml_name.to_string()),
        required,
        r#type: MapperType {
            name: String::from("Composite"),
            className: Some(class_name.to_string()),
            ..MapperType::default()
        },
        ..Mapper::default()
    }
}

pub fn mappers() -> &'static BTreeMap<String, Mapper> {
    &MAPPERS
}

pub fn get_mapper(name: &str) -> Option<&'static Mapper> {
    MAPPERS.get(name)
}
