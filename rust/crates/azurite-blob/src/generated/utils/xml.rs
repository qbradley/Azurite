pub fn stringifyXML(
    obj: &serde_json::Value,
    rootName: Option<&str>,
) -> Result<String, quick_xml::se::SeError> {
    if let Some(rootName) = rootName {
        quick_xml::se::to_string_with_root(rootName, obj)
    } else {
        quick_xml::se::to_string(obj)
    }
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

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let child_val = parse_element(reader)?;

                    // Handle duplicate keys by converting to array
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
    quick_xml::se::to_string(json)
}
