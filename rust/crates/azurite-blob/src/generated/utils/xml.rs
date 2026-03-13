pub fn stringifyXML(
    obj: &serde_json::Value,
    rootName: Option<&str>,
) -> Result<String, quick_xml::se::SeError> {
    let wrapped = if let Some(rootName) = rootName {
        serde_json::json!({ rootName: obj })
    } else {
        obj.clone()
    };
    quick_xml::se::to_string(&wrapped)
}

pub fn parseXML(
    str_value: &str,
    _explicitChildrenWithOrder: bool,
) -> Result<serde_json::Value, quick_xml::DeError> {
    quick_xml::de::from_str(str_value)
}

pub fn jsonToXML(json: &serde_json::Value) -> Result<String, quick_xml::se::SeError> {
    quick_xml::se::to_string(json)
}
