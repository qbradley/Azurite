use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IIPRange {
    pub start: String,
    pub end: Option<String>,
}

#[allow(non_snake_case)]
pub fn ipRangeToString(ipRange: &IIPRange) -> String {
    match &ipRange.end {
        Some(end) if !end.is_empty() => format!("{}-{}", ipRange.start, end),
        _ => ipRange.start.clone(),
    }
}

pub use ipRangeToString as ip_range_to_string;

#[cfg(test)]
mod tests {
    use super::{ipRangeToString, IIPRange};

    #[test]
    fn formats_single_ip_range() {
        let ipRange = IIPRange {
            start: "8.8.8.8".to_owned(),
            end: None,
        };

        assert_eq!(ipRangeToString(&ipRange), "8.8.8.8");
    }

    #[test]
    fn formats_ip_range_with_end() {
        let ipRange = IIPRange {
            start: "1.1.1.1".to_owned(),
            end: Some("255.255.255.255".to_owned()),
        };

        assert_eq!(ipRangeToString(&ipRange), "1.1.1.1-255.255.255.255");
    }
}
