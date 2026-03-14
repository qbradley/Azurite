use thiserror::Error;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IRange {
    pub offset: i64,
    pub count: Option<i64>,
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct RangeError {
    pub message: String,
}

#[allow(non_snake_case)]
pub fn rangeToString(iRange: &IRange) -> Result<String, RangeError> {
    if iRange.offset < 0 {
        return Err(RangeError {
            message: String::from("IRange.offset cannot be smaller than 0."),
        });
    }
    if let Some(count) = iRange.count {
        if count < 0 {
            return Err(RangeError {
                message: String::from(
                    "IRange.count must be larger than 0. Leave it undefined if you want a range from offset to the end.",
                ),
            });
        }
    }
    Ok(match iRange.count {
        Some(count) if count != 0 => {
            format!("bytes={}-{}", iRange.offset, iRange.offset + count - 1)
        }
        _ => format!("bytes={}-", iRange.offset),
    })
}

pub use rangeToString as range_to_string;
