//! Reject repeated JSON members before binding event data or CI configuration.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// RFC 8785's interoperable integer range ends at 2^53 - 1.
/// Larger JSON integers use the runner's binary64 number representation.
pub fn needs_binary64(magnitude: u64) -> (convert: bool)
    ensures convert == (magnitude > 9007199254740991int),
{
    magnitude > 9_007_199_254_740_991
}
}
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
pub fn json(bytes: &[u8]) -> Result<Value, String> {
    let value: serde_yaml_ng::Value =
        serde_json::from_slice(bytes).map_err(|error: serde_json::Error| {
            let message = error.to_string();
            if let Some((quoted, location)) = message
                .strip_prefix("duplicate entry with key ")
                .and_then(|tail| tail.rsplit_once(" at line "))
            {
                if let Ok(key) = litrs::StringLit::parse(quoted) {
                    return format!("duplicate JSON member {} at line {location}", key.value());
                }
            }
            message
        })?;
    serde_json::to_value(value).map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_value(json(bytes)?).map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub fn digest<T: serde::Serialize>(value: &T) -> Result<String, String> {
    Ok(crate::files::sha256(
        &serde_jcs::to_vec(value).map_err(|e| e.to_string())?,
    ))
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn is_digest(s: &str) -> bool {
    s.len() == 64 && crate::verified_literals::hex_content(s.as_bytes()).is_some()
}

/// Present the same IEEE 754 numbers to JCS as the registered runner, including
/// integers that round when represented in binary64. Object keys remain strings.
#[cfg(not(verus_keep_ghost))]
struct WireValue<'a>(&'a Value);

#[cfg(not(verus_keep_ghost))]
impl serde::Serialize for WireValue<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Number(n) => {
                let magnitude = n.as_u64().or_else(|| n.as_i64().map(i64::unsigned_abs));
                if magnitude.is_some_and(needs_binary64) {
                    let number = n.as_f64().ok_or_else(|| {
                        serde::ser::Error::custom("expected a binary64 JSON number")
                    })?;
                    return serializer.serialize_f64(number);
                }
                n.serialize(serializer)
            }
            Value::Array(values) => serializer.collect_seq(values.iter().map(WireValue)),
            Value::Object(values) => {
                serializer.collect_map(values.iter().map(|(key, value)| (key, WireValue(value))))
            }
            value => value.serialize(serializer),
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub fn value_digest(value: &Value) -> Result<String, String> {
    digest(&WireValue(value))
}
