pub fn isURITemplateMatch(url: &str, template: &str) -> bool {
    let mut regex = String::from("^");
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            while let Some(next) = chars.next() {
                if next == '}' {
                    break;
                }
            }
            regex.push_str("[^/]+");
        } else {
            regex.push_str(&regex::escape(&ch.to_string()));
        }
    }
    regex.push('$');
    regex::Regex::new(&regex)
        .map(|compiled| compiled.is_match(url))
        .unwrap_or(false)
}
