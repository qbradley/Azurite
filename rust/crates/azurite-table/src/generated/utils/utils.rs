pub fn isURITemplateMatch(url: &str, template: &str) -> bool {
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Cache compiled regexes to avoid recompiling on every dispatch call.
    static CACHE: std::sync::LazyLock<Mutex<HashMap<String, regex::Regex>>> =
        std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

    let mut cache = CACHE.lock().unwrap();
    let compiled = cache.entry(template.to_string()).or_insert_with(|| {
        let mut pattern = String::from("^");
        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' {
                for next in chars.by_ref() {
                    if next == '}' {
                        break;
                    }
                }
                pattern.push_str("[^/]+");
            } else {
                pattern.push_str(&regex::escape(&ch.to_string()));
            }
        }
        pattern.push('$');
        regex::Regex::new(&pattern).expect("valid regex from URI template")
    });
    compiled.is_match(url)
}

pub fn escapeURLParameter(parameter: &str) -> String {
    parameter
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{:02X}", byte),
        })
        .collect()
}
