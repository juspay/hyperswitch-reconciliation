use std::io::BufRead;

use common_types::transformation::{FixedWidthConfig, FixedWidthConfigV1};

use crate::domain::{
    metadata_schema::types::{FileParsingOutcome, ParsedRow, ParsingDiagnostic, RowParsingOutcome},
    parsing_engine::FileParser,
    transformation::types::{LocatedValue, SourceLocation, SourceRecord},
};

impl FileParser for FixedWidthConfig {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        match self {
            Self::V1(config) => config.parse(input),
        }
    }
}

impl FileParser for FixedWidthConfigV1 {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        let rows = input
            .lines()
            .enumerate()
            .map(|(index, line)| {
                let row = index + 1;
                match line {
                    Ok(line) => {
                        let fields = self
                            .fields
                            .iter()
                            .map(|field| {
                                let value: String =
                                    line.chars().skip(field.index).take(field.width).collect();
                                (
                                    field.header.clone(),
                                    LocatedValue {
                                        value: (!value.is_empty()).then_some(value),
                                        location: SourceLocation::FileRange {
                                            line_number: row,
                                            start_column: field.index + 1,
                                            end_column: field.index + field.width,
                                        },
                                    },
                                )
                            })
                            .collect();
                        RowParsingOutcome::Parsed(ParsedRow {
                            source: SourceRecord::FileRow { line_number: row },
                            fields,
                        })
                    }
                    Err(error) => RowParsingOutcome::Rejected(ParsingDiagnostic {
                        location: SourceLocation::FileRow { line_number: row },
                        message: error.to_string(),
                    }),
                }
            })
            .collect();

        FileParsingOutcome::Parsed { rows }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use common_types::transformation::FixedWidthField;

    fn field(header: &str, index: usize, width: usize) -> FixedWidthField {
        FixedWidthField {
            header: header.to_string(),
            index,
            width,
        }
    }

    fn build_config(fields: Vec<FixedWidthField>) -> FixedWidthConfigV1 {
        FixedWidthConfigV1 { fields }
    }

    fn all_rows(output: &FileParsingOutcome) -> &[RowParsingOutcome] {
        match output {
            FileParsingOutcome::Parsed { rows } => rows,
            FileParsingOutcome::Rejected { failure } => {
                panic!("expected a parsed file, got rejection: {failure:?}")
            }
        }
    }

    fn parsed_rows(output: &FileParsingOutcome) -> Vec<&ParsedRow> {
        all_rows(output)
            .iter()
            .filter_map(|row| match row {
                RowParsingOutcome::Parsed(parsed) => Some(parsed),
                RowParsingOutcome::Rejected(_) => None,
            })
            .collect()
    }

    fn rejected_rows(output: &FileParsingOutcome) -> Vec<&ParsingDiagnostic> {
        all_rows(output)
            .iter()
            .filter_map(|row| match row {
                RowParsingOutcome::Rejected(diagnostic) => Some(diagnostic),
                RowParsingOutcome::Parsed(_) => None,
            })
            .collect()
    }

    #[test]
    fn test_fixed_width_basic_extraction() {
        let config = build_config(vec![field("name", 0, 5), field("amount", 5, 6)]);
        let result = config.parse("John 000100".as_bytes());
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 1);
        assert!(rejected_rows(&result).is_empty());
        assert_eq!(all_rows(&result).len(), 1);

        let fields = &rows[0].fields;
        assert_eq!(
            fields.get("name").map(|field| &field.value),
            Some(&Some("John ".to_string()))
        );
        assert_eq!(
            fields.get("amount").map(|field| &field.value),
            Some(&Some("000100".to_string()))
        );
        assert_eq!(
            fields.get("amount").map(|field| &field.location),
            Some(&SourceLocation::FileRange {
                line_number: 1,
                start_column: 6,
                end_column: 11,
            })
        );
    }

    #[test]
    fn test_fixed_width_line_shorter_than_fields() {
        let config = build_config(vec![field("a", 0, 2), field("b", 5, 3)]);
        let result = config.parse("AB".as_bytes());

        let fields = &parsed_rows(&result)[0].fields;
        assert_eq!(
            fields.get("a").map(|field| &field.value),
            Some(&Some("AB".to_string()))
        );
        assert_eq!(fields.get("b").map(|field| &field.value), Some(&None));
    }

    #[test]
    fn test_fixed_width_variable_widths_with_short_line() {
        let config = build_config(vec![
            field("code", 0, 3),
            field("name", 3, 6),
            field("amount", 9, 5),
        ]);
        let result = config.parse("ABCJohn  00100\nXY7Zz".as_bytes());
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2);
        assert!(rejected_rows(&result).is_empty());
        assert_eq!(all_rows(&result).len(), 2);

        let first = &rows[0].fields;
        assert_eq!(
            first.get("code").map(|field| &field.value),
            Some(&Some("ABC".to_string()))
        );
        assert_eq!(
            first.get("name").map(|field| &field.value),
            Some(&Some("John  ".to_string()))
        );
        assert_eq!(
            first.get("amount").map(|field| &field.value),
            Some(&Some("00100".to_string()))
        );

        let second = &rows[1].fields;
        assert_eq!(
            second.get("code").map(|field| &field.value),
            Some(&Some("XY7".to_string()))
        );
        assert_eq!(
            second.get("name").map(|field| &field.value),
            Some(&Some("Zz".to_string()))
        );
        assert_eq!(second.get("amount").map(|field| &field.value), Some(&None));
    }

    #[test]
    fn test_fixed_width_line_numbers_start_at_one() {
        let config = build_config(vec![field("v", 0, 4)]);
        let result = config.parse("AAAA\nBBBB\nCCCC".as_bytes());
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 3);
        for line_number in [1, 2, 3] {
            assert!(rows
                .iter()
                .any(|row| row.source == SourceRecord::FileRow { line_number }));
        }
    }

    #[test]
    fn test_fixed_width_blank_line_produces_none_row() {
        let config = build_config(vec![field("v", 0, 4)]);
        let result = config.parse("AAAA\n\nBBBB".as_bytes());
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 3);
        assert_eq!(all_rows(&result).len(), 3);

        let blank_row = rows
            .iter()
            .find(|row| row.source == SourceRecord::FileRow { line_number: 2 })
            .unwrap();
        assert_eq!(
            blank_row.fields.get("v").map(|field| &field.value),
            Some(&None)
        );
    }

    #[test]
    fn test_fixed_width_leading_blank_lines_shift_line_numbers() {
        let config = build_config(vec![field("v", 0, 4)]);
        let result = config.parse("\n\nAAAA\nBBBB".as_bytes());
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 4);

        let data_row = rows
            .iter()
            .find(|row| {
                row.fields.get("v").and_then(|field| field.value.as_deref()) == Some("AAAA")
            })
            .unwrap();
        assert_eq!(
            data_row.source,
            SourceRecord::FileRow { line_number: 3 },
            "blank lines are still lines: AAAA sits on line 3"
        );
        assert_eq!(
            data_row.fields.get("v").map(|field| &field.location),
            Some(&SourceLocation::FileRange {
                line_number: 3,
                start_column: 1,
                end_column: 4,
            })
        );
    }

    #[test]
    fn test_fixed_width_columns_are_absolute_not_relative_to_first_content() {
        let config = build_config(vec![field("padding", 0, 10), field("code", 10, 5)]);
        let result = config.parse("          ABCDE".as_bytes());
        let rows = parsed_rows(&result);

        assert_eq!(
            rows[0].fields.get("code").map(|field| &field.value),
            Some(&Some("ABCDE".to_string()))
        );
        assert_eq!(
            rows[0].fields.get("code").map(|field| &field.location),
            Some(&SourceLocation::FileRange {
                line_number: 1,
                start_column: 11,
                end_column: 15,
            }),
            "leading blank columns are never trimmed away"
        );
    }

    #[test]
    fn test_fixed_width_crlf_endings_keep_physical_line_numbers() {
        let config = build_config(vec![field("v", 0, 4)]);
        let result = config.parse("AAAA\r\nBBBB\r\n".as_bytes());
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2, "a trailing terminator adds no phantom row");
        assert_eq!(
            rows[0].fields.get("v").map(|field| &field.value),
            Some(&Some("AAAA".to_string())),
            "the carriage return must not land inside the field value"
        );
        assert_eq!(rows[0].source, SourceRecord::FileRow { line_number: 1 });
        assert_eq!(
            rows[1].fields.get("v").map(|field| &field.value),
            Some(&Some("BBBB".to_string()))
        );
        assert_eq!(rows[1].source, SourceRecord::FileRow { line_number: 2 });
    }

    #[test]
    fn test_fixed_width_empty_file() {
        let config = build_config(vec![field("v", 0, 4)]);
        let result = config.parse(&[]);

        assert_eq!(parsed_rows(&result).len(), 0);
        assert!(rejected_rows(&result).is_empty());
        assert_eq!(all_rows(&result).len(), 0);
    }

    #[test]
    fn test_fixed_width_source_tracking() {
        let config = build_config(vec![field("v", 0, 4)]);
        let result = config.parse("AAAA".as_bytes());

        assert_eq!(
            parsed_rows(&result)[0].source,
            SourceRecord::FileRow { line_number: 1 }
        );
    }
}
