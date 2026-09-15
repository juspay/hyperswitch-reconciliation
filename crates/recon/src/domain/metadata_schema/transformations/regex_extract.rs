use crate::domain::metadata_schema::types::{ConfigFieldValidationError, RuleError};
use error_stack::Report;
use regex::Regex;

pub fn regex_extract_value(
    value: &mut Option<String>,
    pattern: &str,
    group: Option<usize>,
) -> Result<(), Report<RuleError>> {
    let new_value: Option<String> = match value.as_ref() {
        Some(inner) => {
            let compiled = Regex::new(pattern).map_err(|err| RuleError::InvalidConfig {
                details: err.to_string(),
            })?;

            let captures = compiled.captures(inner).ok_or(RuleError::UnmatchedRegex {
                pattern: pattern.to_string(),
            })?;

            let matched = match group {
                Some(g) => captures.get(g),
                None => captures.get(0),
            };
            let extracted =
                matched
                    .map(|m| m.as_str().to_string())
                    .ok_or(RuleError::UnmatchedRegex {
                        pattern: pattern.to_string(),
                    })?;

            Some(extracted)
        }
        None => None,
    };
    *value = new_value;
    Ok(())
}

pub fn validate_regex_config(
    pattern: &str,
    group: Option<usize>,
) -> Result<(), Report<ConfigFieldValidationError>> {
    let compiled =
        Regex::new(pattern).map_err(|err| ConfigFieldValidationError::RegexCompileFailed {
            pattern: pattern.to_string(),
            reason: err.to_string(),
        })?;

    if let Some(g) = group {
        let capture_count = compiled.captures_len().saturating_sub(1);
        if g > capture_count {
            return Err(ConfigFieldValidationError::RegexGroupOutOfRange {
                pattern: pattern.to_string(),
                group: g,
                capture_count,
            }
            .into());
        }
    }

    Ok(())
}
