//! Custom validations for some shared types.

use std::{
    collections::HashSet,
    path::{Component, Path},
    sync::LazyLock,
};

use error_stack::report;
use globset::Glob;
use regex::Regex;

use crate::errors::{CustomResult, ValidationError};

/// Validates a given phone number using the [phonenumber] crate
///
/// It returns a [ValidationError::InvalidValue] in case it could not parse the phone number
pub fn validate_phone_number(phone_number: &str) -> Result<(), ValidationError> {
    let _ = phonenumber::parse(None, phone_number).map_err(|e| ValidationError::InvalidValue {
        message: format!("Could not parse phone number: {phone_number}, because: {e:?}"),
    })?;

    Ok(())
}

/// Performs a simple validation against a provided email address.
pub fn validate_email(email: &str) -> CustomResult<(), ValidationError> {
    #[deny(clippy::invalid_regex)]
    static EMAIL_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| {
        #[allow(unknown_lints)]
        #[allow(clippy::manual_ok_err)]
        match Regex::new(
            r"^(?i)[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+$",
        ) {
            Ok(regex) => Some(regex),
            Err(_error) => None,
        }
    });
    let email_regex = match EMAIL_REGEX.as_ref() {
        Some(regex) => Ok(regex),
        None => Err(report!(ValidationError::InvalidValue {
            message: "Invalid regex expression".into()
        })),
    }?;

    const EMAIL_MAX_LENGTH: usize = 319;
    if email.is_empty() || email.chars().count() > EMAIL_MAX_LENGTH {
        return Err(report!(ValidationError::InvalidValue {
            message: "Email address is either empty or exceeds maximum allowed length".into()
        }));
    }

    if !email_regex.is_match(email) {
        return Err(report!(ValidationError::InvalidValue {
            message: "Invalid email address format".into()
        }));
    }

    Ok(())
}

/// Checks whether a given domain matches against a list of valid domain glob patterns
pub fn validate_domain_against_allowed_domains(
    domain: &str,
    allowed_domains: HashSet<String>,
) -> bool {
    allowed_domains.iter().any(|allowed_domain| {
        Glob::new(allowed_domain)
            .map(|glob| glob.compile_matcher().is_match(domain))
            .map_err(|err| {
                let err_msg = format!(
                    "Invalid glob pattern for configured allowed_domain [{allowed_domain:?}]! - {err:?}",

                );
                err_msg
            })
            .unwrap_or(false)
    })
}

pub fn validate_sftp_file_path(file_path: &Path) -> CustomResult<(), ValidationError> {
    let path = file_path.to_str().ok_or_else(|| {
        report!(ValidationError::IncorrectValueProvided {
            field_name: "file_path",
        })
        .attach_printable("SFTP file_path must be valid UTF-8")
    })?;
    if path.is_empty()
        || path.contains(['\\', '\0'])
        || file_path.components().any(|component| {
            matches!(
                component,
                Component::RootDir | Component::Prefix(_) | Component::ParentDir
            )
        })
        || matches!(path.rsplit('/').next(), None | Some("" | "." | ".."))
    {
        return Err(report!(ValidationError::IncorrectValueProvided {
            field_name: "file_path",
        })
        .attach_printable("SFTP file_path must be a relative file path without parent-directory components or backslashes"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use fake::{faker::internet::en::SafeEmail, Fake};
    use proptest::{
        prop_assert,
        strategy::{Just, NewTree, Strategy},
        test_runner::TestRunner,
    };
    use test_case::test_case;

    use super::*;

    #[derive(Debug)]
    struct ValidEmail;

    impl Strategy for ValidEmail {
        type Tree = Just<String>;
        type Value = String;

        fn new_tree(&self, _runner: &mut TestRunner) -> NewTree<Self> {
            Ok(Just(SafeEmail().fake()))
        }
    }

    #[test]
    fn test_validate_email() {
        let result = validate_email("abc@example.com");
        assert!(result.is_ok());

        let result = validate_email("abc+123@example.com");
        assert!(result.is_ok());

        let result = validate_email("");
        assert!(result.is_err());
    }

    #[test_case("+40745323456" ; "Romanian valid phone number")]
    #[test_case("+34912345678" ; "Spanish valid phone number")]
    #[test_case("+41 79 123 45 67" ; "Swiss valid phone number")]
    #[test_case("+66 81 234 5678" ; "Thailand valid phone number")]
    fn test_validate_phone_number(phone_number: &str) {
        assert!(validate_phone_number(phone_number).is_ok());
    }

    #[test_case("9123456789" ; "Romanian invalid phone number")]
    fn test_invalid_phone_number(phone_number: &str) {
        let res = validate_phone_number(phone_number);
        assert!(res.is_err());
    }

    proptest::proptest! {
        /// Example of unit test
        #[test]
        fn proptest_valid_fake_email(email in ValidEmail) {
            prop_assert!(validate_email(&email).is_ok());
        }

        /// Example of unit test
        #[test]
        fn proptest_invalid_data_email(email in "\\PC*") {
            prop_assert!(validate_email(&email).is_err());
        }

        #[test]
        fn proptest_invalid_email(email in "[.+]@(.+)") {
            prop_assert!(validate_email(&email).is_err());
        }
    }

    #[test_case("report.csv"; "filename")]
    #[test_case("reports/2026/daily.csv"; "nested directories")]
    #[test_case(".report.csv"; "hidden filename")]
    #[test_case("reports/report..csv"; "dots within filename")]
    #[test_case("reports/daily report.csv"; "spaces in filename")]
    #[test_case("reports/日次.csv"; "unicode filename")]
    #[test_case("sftp/other_merchant/report.csv"; "prefix looking relative path")]
    fn test_validate_sftp_file_path_accepts_relative_files(path: &str) {
        assert!(validate_sftp_file_path(Path::new(path)).is_ok());
    }

    #[test_case(""; "empty path")]
    #[test_case("/"; "root directory")]
    #[test_case("/reports/daily.csv"; "absolute path")]
    #[test_case("../other_merchant/report.csv"; "leading parent directory")]
    #[test_case("reports/../../other_merchant/report.csv"; "nested traversal")]
    #[test_case("reports/../report.csv"; "parent directory within merchant")]
    #[test_case("reports/.."; "trailing parent directory")]
    #[test_case("reports\\daily.csv"; "backslash separator")]
    #[test_case("..\\other_merchant\\report.csv"; "backslash traversal")]
    #[test_case("C:\\reports\\daily.csv"; "windows absolute path")]
    #[test_case("report\0.csv"; "null byte")]
    #[test_case("."; "current directory")]
    #[test_case(".."; "parent directory")]
    #[test_case("reports/"; "trailing slash")]
    #[test_case("reports/."; "trailing current directory")]
    fn test_validate_sftp_file_path_rejects_invalid_paths(path: &str) {
        let result = validate_sftp_file_path(Path::new(path));
        assert_eq!(
            result.as_ref().err().map(|error| error.current_context()),
            Some(&ValidationError::IncorrectValueProvided {
                field_name: "file_path",
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_sftp_file_path_rejects_non_utf8() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let path = Path::new(OsStr::from_bytes(b"reports/\xff.csv"));
        let result = validate_sftp_file_path(path);
        assert_eq!(
            result.as_ref().err().map(|error| error.current_context()),
            Some(&ValidationError::IncorrectValueProvided {
                field_name: "file_path",
            })
        );
    }
}
