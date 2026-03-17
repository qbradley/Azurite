const XML_DECLARATION: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

fn with_xml_declaration(body: String) -> String {
    format!("{XML_DECLARATION}{body}")
}

pub fn stringifyXML(
    obj: &serde_json::Value,
    rootName: Option<&str>,
) -> Result<String, quick_xml::se::SeError> {
    let xml = if let Some(rootName) = rootName {
        quick_xml::se::to_string_with_root(rootName, obj)
    } else {
        quick_xml::se::to_string(obj)
    }?;

    Ok(with_xml_declaration(xml))
}

/// Parse XML string to serde_json::Value, handling duplicate sibling elements
/// by collecting them into arrays (matching xml2js behavior with explicitArray: false).
pub fn parseXML(
    str_value: &str,
    _explicitChildrenWithOrder: bool,
) -> Result<serde_json::Value, quick_xml::DeError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    use serde_json::{Map, Value};

    let mut reader = Reader::from_str(str_value);
    reader.config_mut().trim_text(true);

    fn parse_element(reader: &mut Reader<&[u8]>) -> Result<Value, quick_xml::DeError> {
        let mut children: Map<String, Value> = Map::new();
        let mut text = String::new();

        fn insert_child(children: &mut Map<String, Value>, name: String, child_val: Value) {
            if let Some(existing) = children.remove(&name) {
                match existing {
                    Value::Array(mut arr) => {
                        arr.push(child_val);
                        children.insert(name, Value::Array(arr));
                    }
                    _ => {
                        children.insert(name, Value::Array(vec![existing, child_val]));
                    }
                }
            } else {
                children.insert(name, child_val);
            }
        }

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let child_val = parse_element(reader)?;
                    insert_child(&mut children, name, child_val);
                }
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    insert_child(&mut children, name, Value::String(String::new()));
                }
                Ok(Event::Text(e)) => {
                    let t = e.unescape().map_err(|err| {
                        quick_xml::DeError::Custom(format!("text unescape error: {}", err))
                    })?;
                    text.push_str(&t);
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(quick_xml::DeError::Custom(format!(
                        "XML parse error: {}",
                        e
                    )))
                }
            }
        }

        if children.is_empty() {
            Ok(Value::String(text))
        } else {
            Ok(Value::Object(children))
        }
    }

    // Skip to root element
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => {
                return parse_element(&mut reader);
            }
            Ok(Event::Empty(_)) => {
                // Self-closing root: <BlockList/> → empty object
                return Ok(Value::Object(Map::new()));
            }
            Ok(Event::Decl(_)) | Ok(Event::Comment(_)) | Ok(Event::PI(_)) => continue,
            Ok(Event::Eof) => return Ok(Value::Null),
            Ok(_) => continue,
            Err(e) => {
                return Err(quick_xml::DeError::Custom(format!(
                    "XML parse error: {}",
                    e
                )))
            }
        }
    }
}

pub fn jsonToXML(json: &serde_json::Value) -> Result<String, quick_xml::se::SeError> {
    quick_xml::se::to_string(json).map(with_xml_declaration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multiple_signed_identifiers() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><SignedIdentifiers><SignedIdentifier><Id>policy1</Id><AccessPolicy><Start>2020-01-01T00:00:00Z</Start><Expiry>2025-01-01T00:00:00Z</Expiry><Permission>raup</Permission></AccessPolicy></SignedIdentifier><SignedIdentifier><Id>policy2</Id><AccessPolicy><Start>2020-01-01T00:00:00Z</Start><Expiry>2025-01-01T00:00:00Z</Expiry><Permission>raup</Permission></AccessPolicy></SignedIdentifier><SignedIdentifier><Id>policy3</Id><AccessPolicy><Start>2020-01-01T00:00:00Z</Start><Expiry>2025-01-01T00:00:00Z</Expiry><Permission>raup</Permission></AccessPolicy></SignedIdentifier></SignedIdentifiers>"#;

        let parsed = parseXML(xml, false).unwrap();
        let si = parsed.get("SignedIdentifier").unwrap();
        assert!(si.is_array(), "Expected array, got {:?}", si);
        assert_eq!(si.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_parse_single_element_stays_object() {
        let xml = r#"<Root><Item>hello</Item></Root>"#;
        let parsed = parseXML(xml, false).unwrap();
        let item = parsed.get("Item").unwrap();
        assert!(item.is_string());
        assert_eq!(item.as_str().unwrap(), "hello");
    }

    #[test]
    fn stringify_xml_includes_xml2js_declaration() {
        let xml = stringifyXML(&serde_json::json!({ "Item": "hello" }), Some("Root")).unwrap();
        assert_eq!(
            xml,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Root><Item>hello</Item></Root>"#
        );
    }

    #[test]
    fn json_to_xml_includes_xml2js_declaration() {
        let xml = stringifyXML(&serde_json::json!({ "Item": "hello" }), Some("Root")).unwrap();
        assert_eq!(
            xml,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Root><Item>hello</Item></Root>"#
        );
    }

    #[test]
    fn stringify_xml_serializes_at_prefixed_keys_as_attributes() {
        let xml = stringifyXML(
            &serde_json::json!({
                "@ServiceEndpoint": "http://127.0.0.1/devstoreaccount1",
                "@ContainerName": "mycontainer",
                "Blobs": { "Blob": [] }
            }),
            Some("EnumerationResults"),
        )
        .unwrap();

        assert!(xml.contains("<EnumerationResults"));
        assert!(xml.contains("ServiceEndpoint=\"http://127.0.0.1/devstoreaccount1\""));
        assert!(xml.contains("ContainerName=\"mycontainer\""));
        assert!(xml.contains("<Blobs/>") || xml.contains("<Blobs></Blobs>"));
    }
}
