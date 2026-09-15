use common_types::transformation::{CsvParsingConfig, CsvParsingConfigV1};
use csv;

use crate::domain::{
    metadata_schema::types::{
        FileParsingOutcome, ParsedRow, ParsingDiagnostic, ParsingFailure, RowParsingOutcome,
    },
    parsing_engine::FileParser,
    transformation::types::{LocatedValue, SourceLocation, SourceRecord},
};

impl FileParser for CsvParsingConfig {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        match self {
            Self::V1(config) => config.parse(input),
        }
    }
}

impl FileParser for CsvParsingConfigV1 {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(input);
        let headers = match reader.headers() {
            Ok(headers) => headers.clone(),
            Err(error) => {
                return FileParsingOutcome::rejected(ParsingFailure::InvalidFormat {
                    details: error.to_string(),
                })
            }
        };

        let rows = reader
            .records()
            .enumerate()
            .map(|(index, record)| match record {
                Ok(record) => {
                    let row = csv_record_line_number(input, record.position(), index + 2);
                    let fields = headers
                        .iter()
                        .enumerate()
                        .map(|(column, field_name)| {
                            (
                                field_name.to_string(),
                                LocatedValue {
                                    // An empty cell is absence, not an empty string — skip
                                    // configs and required-field checks depend on the difference.
                                    value: record
                                        .get(column)
                                        .filter(|value| !value.is_empty())
                                        .map(str::to_string),
                                    location: SourceLocation::FileCell {
                                        line_number: row,
                                        column: column + 1,
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
                Err(error) => {
                    let row = csv_record_line_number(input, error.position(), index + 2);
                    RowParsingOutcome::Rejected(ParsingDiagnostic {
                        location: SourceLocation::FileRow { line_number: row },
                        message: error.to_string(),
                    })
                }
            })
            .collect();

        FileParsingOutcome::Parsed { rows }
    }
}

fn csv_record_line_number(
    input: &[u8],
    position: Option<&csv::Position>,
    fallback: usize,
) -> usize {
    position
        .and_then(|position| {
            let line = usize::try_from(position.line()).ok()?;
            let byte = usize::try_from(position.byte()).ok()?;
            let pending_line_feeds = input
                .get(byte..)?
                .iter()
                .take_while(|byte| matches!(**byte, b'\r' | b'\n'))
                .filter(|byte| **byte == b'\n')
                .count();
            Some(line + pending_line_feeds)
        })
        .unwrap_or(fallback)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::indexing_slicing,
    clippy::get_first,
    clippy::panic
)]
mod tests {
    use super::*;
    use rstest::rstest;

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

    fn create_expected_parsing_result(
        parsed_row_fields: Vec<Vec<(&str, &str)>>,
        errors: Vec<(usize, &str)>,
    ) -> FileParsingOutcome {
        let mut rows = Vec::new();

        for (i, row_fields) in parsed_row_fields.iter().enumerate() {
            // Line mapping: 1 for header + (i + 1) for data row
            let line_number = i + 2;
            let fields = row_fields
                .iter()
                .enumerate()
                .map(|(column, (field_name, value))| {
                    (
                        field_name.to_string(),
                        LocatedValue {
                            value: Some(value.to_string()),
                            location: SourceLocation::FileCell {
                                line_number,
                                column: column + 1,
                            },
                        },
                    )
                })
                .collect();
            rows.push(RowParsingOutcome::Parsed(ParsedRow {
                source: SourceRecord::FileRow { line_number },
                fields,
            }));
        }

        for (line_number, error) in errors {
            rows.push(RowParsingOutcome::Rejected(ParsingDiagnostic {
                location: SourceLocation::FileRow { line_number },
                message: error.to_string(),
            }));
        }

        FileParsingOutcome::Parsed { rows }
    }

    #[rstest]
    #[case(
        "name,age,city\nJohn,25,NYC\nJane,30,LA",
        create_expected_parsing_result(
            vec![
                vec![("name", "John"), ("age", "25"), ("city", "NYC")],
                vec![("name", "Jane"), ("age", "30"), ("city", "LA")]
            ],
            vec![]
        )
    )]
    #[case(
        "id,product,price\n1,Apple,1.50\n2,Banana,0.75",
        create_expected_parsing_result(
            vec![
                vec![("id", "1"), ("product", "Apple"), ("price", "1.50")],
                vec![("id", "2"), ("product", "Banana"), ("price", "0.75")]
            ],
            vec![]
        )
    )]
    #[case(
        "name,age\nJohn,25\nJane", // Missing field in flexible mode - parses with age=None
        create_expected_parsing_result(
            vec![
                vec![("name", "John"), ("age", "25")],
                vec![("name", "Jane")]
            ],
            vec![]
        )
    )]
    #[case(
        "", // Empty file
        create_expected_parsing_result(vec![], vec![])
    )]
    #[case(
        "name,age,city", // only header
        create_expected_parsing_result(
            vec![],
            vec![]
        )
    )]
    #[case(
        "name,description,price\n\"John Doe\",\"A product, with comma\",25.50\n\"Jane Smith\",\"Another item, also with comma\",30.00",
        create_expected_parsing_result(
            vec![
                vec![("name", "John Doe"), ("description", "A product, with comma"), ("price", "25.50")],
                vec![("name", "Jane Smith"), ("description", "Another item, also with comma"), ("price", "30.00")]
            ],
            vec![]
        )
    )]
    fn test_csv_parsing(#[case] csv_data: &str, #[case] expected_result: FileParsingOutcome) {
        let config = CsvParsingConfigV1;
        let result = config.parse(csv_data.as_bytes());

        let actual_parsed = parsed_rows(&result);
        let expected_parsed = parsed_rows(&expected_result);
        let actual_rejected = rejected_rows(&result);
        let expected_rejected = rejected_rows(&expected_result);

        // Compare basic metrics
        assert_eq!(all_rows(&result).len(), all_rows(&expected_result).len());
        assert_eq!(actual_parsed.len(), expected_parsed.len());
        assert_eq!(actual_rejected.len(), expected_rejected.len());

        // Compare parsed rows
        for (i, (actual_row, expected_row)) in
            actual_parsed.iter().zip(expected_parsed.iter()).enumerate()
        {
            for (key, expected_value) in &expected_row.fields {
                let actual_value = actual_row.fields.get(key).map(|field| &field.value);
                assert_eq!(
                    actual_value,
                    Some(&expected_value.value),
                    "Row {i} field '{key}' doesn't match"
                );
            }
        }

        // Compare errors (just count and locations, not exact messages)
        for (i, (actual_error, expected_error)) in actual_rejected
            .iter()
            .zip(expected_rejected.iter())
            .enumerate()
        {
            assert_eq!(
                actual_error.location, expected_error.location,
                "Error {i} location doesn't match"
            );
        }

        // Verify each parsed row carries its own file line number
        for (i, actual_row) in actual_parsed.iter().enumerate() {
            let line_number = i + 2;
            assert_eq!(
                actual_row.source,
                SourceRecord::FileRow { line_number },
                "Missing line mapping for line {line_number}"
            );
        }
    }

    #[test]
    fn csv_crlf_line_numbers_start_after_header() {
        let result = CsvParsingConfigV1.parse(b"name,amount\r\nJohn,100\r\nJane,200\r\n");
        let rows = parsed_rows(&result);

        assert_eq!(
            rows.iter().map(|row| &row.source).collect::<Vec<_>>(),
            vec![
                &SourceRecord::FileRow { line_number: 2 },
                &SourceRecord::FileRow { line_number: 3 },
            ]
        );
    }

    #[test]
    fn csv_crlf_multiline_records_use_physical_start_lines() {
        let result =
            CsvParsingConfigV1.parse(b"name,amount\r\n\"John\r\nDoe\",100\r\nJane,200\r\n");
        let rows = parsed_rows(&result);

        assert_eq!(
            rows.iter().map(|row| &row.source).collect::<Vec<_>>(),
            vec![
                &SourceRecord::FileRow { line_number: 2 },
                &SourceRecord::FileRow { line_number: 4 },
            ]
        );
    }

    #[test]
    fn csv_empty_cell_is_none_not_empty_string() {
        let result = CsvParsingConfigV1.parse(b"name,note\nJohn,\n");
        let rows = parsed_rows(&result);

        assert_eq!(
            rows[0].fields.get("note").map(|field| &field.value),
            Some(&None),
            "an empty cell must read as absent, not as Some(\"\")"
        );
        assert_eq!(
            rows[0].fields.get("name").map(|field| &field.value),
            Some(&Some("John".to_string()))
        );
    }

    #[test]
    fn csv_fields_include_source_locations() {
        let result = CsvParsingConfigV1.parse(b"name,amount\nJohn,100");
        let rows = parsed_rows(&result);
        let row = rows[0];

        assert_eq!(row.source, SourceRecord::FileRow { line_number: 2 });
        assert_eq!(
            row.fields.get("amount").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 2,
                column: 2,
            })
        );
    }
}
