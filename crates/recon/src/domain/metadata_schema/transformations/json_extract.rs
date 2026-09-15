use crate::domain::metadata_schema::types::RuleError;
use error_stack::Report;

pub fn json_extract_value(
    value: &mut Option<String>,
    identifier: &str,
) -> Result<(), Report<RuleError>> {
    let new_value: Option<String> = match value.as_ref() {
        Some(inner) => {
            let json: serde_json::Value =
                serde_json::from_str(inner).map_err(|_| RuleError::JsonParseFailed)?;

            let extracted = json.pointer(identifier).ok_or(RuleError::JsonKeyNotFound {
                key: identifier.to_string(),
            })?;

            match &extracted {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(_)
                | serde_json::Value::Bool(_)
                | serde_json::Value::Array(_)
                | serde_json::Value::Object(_) => Some(extracted.to_string()),
                serde_json::Value::Null => None,
            }
        }
        None => None,
    };
    *value = new_value;
    Ok(())
}
