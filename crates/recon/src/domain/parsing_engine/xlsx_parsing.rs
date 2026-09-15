use std::io::{BufReader, Cursor};

use calamine::{Data, Reader, Xlsx};
use common_types::transformation::{SheetSelection, XlsxParsingConfig, XlsxParsingConfigV1};

use crate::domain::{
    metadata_schema::types::{FileParsingOutcome, ParsedRow, ParsingFailure, RowParsingOutcome},
    parsing_engine::FileParser,
    transformation::types::{LocatedValue, SourceLocation, SourceRecord},
};

impl FileParser for XlsxParsingConfig {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        match self {
            Self::V1(config) => config.parse(input),
        }
    }
}

impl FileParser for XlsxParsingConfigV1 {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        let cursor = Cursor::new(input);
        let mut workbook: Xlsx<_> = match Xlsx::new(BufReader::new(cursor)) {
            Ok(workbook) => workbook,
            Err(error) => {
                return FileParsingOutcome::rejected(ParsingFailure::InvalidFormat {
                    details: format!("Failed to open XLSX file: {error}"),
                })
            }
        };
        let range = match &self.sheet_selection {
            SheetSelection::ByIndex(index) => match workbook.worksheet_range_at(*index) {
                Some(Ok(range)) => range,
                Some(Err(error)) => {
                    return FileParsingOutcome::rejected(ParsingFailure::InvalidFormat {
                        details: error.to_string(),
                    })
                }
                None => return FileParsingOutcome::rejected(ParsingFailure::SheetNotFound),
            },
            SheetSelection::ByName(name) => match workbook.worksheet_range(name) {
                Ok(range) => range,
                Err(err) => {
                    return FileParsingOutcome::rejected(ParsingFailure::InvalidFormat {
                        details: err.to_string(),
                    })
                }
            },
        };
        let (row_offset, column_offset) = range
            .start()
            .map(|(row, column)| {
                (
                    usize::try_from(row).unwrap_or(0),
                    usize::try_from(column).unwrap_or(0),
                )
            })
            .unwrap_or_default();
        let rows: Vec<_> = range.rows().collect();
        let Some(header_row) = rows.get(self.header_row_index) else {
            return FileParsingOutcome::rejected(ParsingFailure::HeaderNotFound);
        };
        let headers: Vec<(String, usize)> = header_row
            .iter()
            .enumerate()
            .filter_map(|(column, cell)| data_to_option_string(cell).map(|name| (name, column)))
            .collect();

        let rows = rows
            .iter()
            .enumerate()
            .skip(self.header_row_index + 1)
            .map(|(row_index, row)| {
                let row_number = row_offset + row_index + 1;
                let fields = headers
                    .iter()
                    .map(|(name, column)| {
                        (
                            name.clone(),
                            LocatedValue {
                                value: row.get(*column).and_then(data_to_option_string),
                                location: SourceLocation::FileCell {
                                    line_number: row_number,
                                    column: column_offset + column + 1,
                                },
                            },
                        )
                    })
                    .collect();
                RowParsingOutcome::Parsed(ParsedRow {
                    source: SourceRecord::FileRow {
                        line_number: row_number,
                    },
                    fields,
                })
            })
            .collect();

        FileParsingOutcome::Parsed { rows }
    }
}

fn data_to_option_string(data: &Data) -> Option<String> {
    match data {
        Data::Empty => None,
        Data::String(value) if value.is_empty() => None,
        Data::String(value) => Some(value.clone()),
        Data::Int(value) => Some(value.to_string()),
        Data::Float(value) => Some(format!("{value}")),
        Data::Bool(value) => Some(value.to_string()),
        Data::DateTimeIso(value) | Data::DurationIso(value) => Some(value.clone()),
        Data::Error(error) => Some(format!("{error:?}")),
        Data::DateTime(value) => Some(format!("{value:?}")),
    }
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
    use crate::domain::metadata_schema::types::ParsingDiagnostic;
    use rust_xlsxwriter::Workbook;

    fn create_simple_xlsx(headers: &[&str], rows: &[Vec<&str>]) -> Vec<u8> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        for (col, header) in headers.iter().enumerate() {
            worksheet.write_string(0, col as u16, *header).unwrap();
        }

        for (row_idx, row) in rows.iter().enumerate() {
            for (col, value) in row.iter().enumerate() {
                worksheet
                    .write_string((row_idx + 1) as u32, col as u16, *value)
                    .unwrap();
            }
        }

        workbook.save_to_buffer().unwrap().to_vec()
    }

    fn create_xlsx_with_title_rows(
        title_rows: &[Vec<&str>],
        headers: &[&str],
        data_rows: &[Vec<&str>],
    ) -> Vec<u8> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        let mut row_num: u32 = 0;

        for row in title_rows {
            for (col, value) in row.iter().enumerate() {
                worksheet.write_string(row_num, col as u16, *value).unwrap();
            }
            row_num += 1;
        }

        for (col, header) in headers.iter().enumerate() {
            worksheet
                .write_string(row_num, col as u16, *header)
                .unwrap();
        }
        row_num += 1;

        for row in data_rows {
            for (col, value) in row.iter().enumerate() {
                worksheet.write_string(row_num, col as u16, *value).unwrap();
            }
            row_num += 1;
        }

        workbook.save_to_buffer().unwrap().to_vec()
    }

    fn create_multi_sheet_xlsx(
        sheet1_name: &str,
        sheet1_data: &[Vec<&str>],
        sheet2_name: &str,
        sheet2_data: &[Vec<&str>],
    ) -> Vec<u8> {
        let mut workbook = Workbook::new();

        let worksheet1 = workbook.add_worksheet().set_name(sheet1_name).unwrap();
        for (row_idx, row) in sheet1_data.iter().enumerate() {
            for (col, value) in row.iter().enumerate() {
                worksheet1
                    .write_string(row_idx as u32, col as u16, *value)
                    .unwrap();
            }
        }

        let worksheet2 = workbook.add_worksheet().set_name(sheet2_name).unwrap();
        for (row_idx, row) in sheet2_data.iter().enumerate() {
            for (col, value) in row.iter().enumerate() {
                worksheet2
                    .write_string(row_idx as u32, col as u16, *value)
                    .unwrap();
            }
        }

        workbook.save_to_buffer().unwrap().to_vec()
    }

    fn create_xlsx_at_offset(row_offset: u32, column_offset: u16, rows: &[Vec<&str>]) -> Vec<u8> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                if value.is_empty() {
                    continue;
                }
                worksheet
                    .write_string(
                        row_offset + row_index as u32,
                        column_offset + column_index as u16,
                        *value,
                    )
                    .unwrap();
            }
        }

        workbook.save_to_buffer().unwrap().to_vec()
    }

    fn create_empty_xlsx() -> Vec<u8> {
        let mut workbook = Workbook::new();
        workbook.add_worksheet();
        workbook.save_to_buffer().unwrap().to_vec()
    }

    fn create_header_only_xlsx(headers: &[&str]) -> Vec<u8> {
        create_simple_xlsx(headers, &[])
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

    fn has_line(rows: &[&ParsedRow], line_number: usize) -> bool {
        rows.iter()
            .any(|row| row.source == SourceRecord::FileRow { line_number })
    }

    // ---- XLSX Parsing Tests ----

    #[test]
    fn test_xlsx_simple_parsing() {
        let xlsx_data = create_simple_xlsx(
            &["name", "age", "city"],
            &[vec!["John", "25", "NYC"], vec!["Jane", "30", "LA"]],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2);
        assert!(rejected_rows(&result).is_empty());
        assert_eq!(all_rows(&result).len(), 2);

        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("name")
                .map(|field| &field.value),
            Some(&Some("John".to_string()))
        );
        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("age")
                .map(|field| &field.value),
            Some(&Some("25".to_string()))
        );
        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("city")
                .map(|field| &field.value),
            Some(&Some("NYC".to_string()))
        );
        assert_eq!(
            rows[0].fields.get("city").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 2,
                column: 3,
            })
        );

        assert!(has_line(&rows, 2));
        assert!(has_line(&rows, 3));
    }

    #[test]
    fn test_xlsx_header_row_offset() {
        let xlsx_data = create_xlsx_with_title_rows(
            &[vec!["Bank Statement"], vec!["Account: 12345"]],
            &[
                "Transaction Date",
                "Narrative",
                "Debit Amount",
                "Credit Amount",
            ],
            &[
                vec!["2026-03-25", "Payment from XYZ", "", "1500.00"],
                vec!["2026-03-26", "Transfer to ABC", "500.00", ""],
            ],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 2,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2);
        assert_eq!(rejected_rows(&result).len(), 0);

        assert!(rows.get(0).unwrap().fields.contains_key("Transaction Date"));
        assert!(rows.get(0).unwrap().fields.contains_key("Narrative"));

        assert!(has_line(&rows, 4));
        assert!(has_line(&rows, 5));
    }

    #[test]
    fn test_xlsx_offset_sheet_reports_absolute_cell_coordinates() {
        let xlsx_data = create_xlsx_at_offset(
            2,
            2,
            &[vec!["name", "age"], vec!["John", "25"], vec!["Jane", "30"]],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2);
        assert!(rejected_rows(&result).is_empty());

        assert_eq!(
            rows[0].source,
            SourceRecord::FileRow { line_number: 4 },
            "the table starts at C3, so its first data row is Excel row 4"
        );
        assert_eq!(
            rows[0].fields.get("name").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 4,
                column: 3,
            }),
            "the 'name' column is Excel column C, not A"
        );
        assert_eq!(
            rows[0].fields.get("age").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 4,
                column: 4,
            })
        );
        assert_eq!(
            rows[1].fields.get("age").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 5,
                column: 4,
            })
        );

        assert_eq!(
            rows[0].fields.get("age").map(|field| &field.value),
            Some(&Some("25".to_string())),
            "values must still line up with the shifted columns"
        );
        assert_eq!(
            rows[1].fields.get("name").map(|field| &field.value),
            Some(&Some("Jane".to_string()))
        );
    }

    #[test]
    fn test_xlsx_blank_leading_columns_only_shift_column_numbers() {
        let xlsx_data = create_xlsx_at_offset(0, 3, &[vec!["name", "age"], vec!["John", "25"]]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows[0].source, SourceRecord::FileRow { line_number: 2 });
        assert_eq!(
            rows[0].fields.get("name").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 2,
                column: 4,
            })
        );
        assert_eq!(
            rows[0].fields.get("age").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 2,
                column: 5,
            })
        );
    }

    #[test]
    fn test_xlsx_blank_leading_rows_only_shift_line_numbers() {
        let xlsx_data = create_xlsx_at_offset(4, 0, &[vec!["name", "age"], vec!["John", "25"]]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows[0].source, SourceRecord::FileRow { line_number: 6 });
        assert_eq!(
            rows[0].fields.get("name").map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 6,
                column: 1,
            })
        );
    }

    #[test]
    fn test_xlsx_header_row_index_is_relative_to_trimmed_range() {
        let xlsx_data = create_xlsx_at_offset(
            5,
            1,
            &[
                vec!["Bank Statement"],
                vec!["Transaction Date", "Credit Amount"],
                vec!["2026-03-25", "1500.00"],
                vec!["2026-03-26", "500.00"],
            ],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 1,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].fields.contains_key("Transaction Date"));

        assert!(
            has_line(&rows, 8),
            "header_row_index counts rows calamine returned, but the reported line is absolute"
        );
        assert!(has_line(&rows, 9));
        assert_eq!(
            rows[0]
                .fields
                .get("Credit Amount")
                .map(|field| &field.location),
            Some(&SourceLocation::FileCell {
                line_number: 8,
                column: 3,
            })
        );
    }

    #[test]
    fn test_xlsx_empty_cells() {
        let xlsx_data = create_simple_xlsx(
            &["name", "age", "city"],
            &[vec!["John", "25", "NYC"], vec!["Jane", "", ""]],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.get(1)
                .unwrap()
                .fields
                .get("age")
                .map(|field| &field.value),
            Some(&None)
        );
        assert_eq!(
            rows.get(1)
                .unwrap()
                .fields
                .get("city")
                .map(|field| &field.value),
            Some(&None)
        );
    }

    #[test]
    fn test_xlsx_skips_empty_header_columns() {
        // Some bank exports use blank spacer columns between logical groups
        // (e.g., an account statement with blanks at cols 3, 6, 9). The parser
        // now silently skips those columns — same spirit as CSV's flexible mode.
        // Data under skipped columns is dropped; data under named columns is kept.
        let xlsx_data = create_simple_xlsx(
            &["name", "", "age", "", "city"],
            &[
                vec!["John", "ignored_1", "25", "ignored_2", "NYC"],
                vec!["Jane", "ignored_3", "30", "ignored_4", "LA"],
            ],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 2);
        assert!(rejected_rows(&result).is_empty());

        let row0_fields = &rows[0].fields;
        assert_eq!(
            row0_fields.get("name").map(|field| &field.value),
            Some(&Some("John".to_string()))
        );
        assert_eq!(
            row0_fields.get("age").map(|field| &field.value),
            Some(&Some("25".to_string()))
        );
        assert_eq!(
            row0_fields.get("city").map(|field| &field.value),
            Some(&Some("NYC".to_string()))
        );
        // Only the three named headers should appear; empty-header columns drop out.
        assert_eq!(row0_fields.len(), 3);
        assert!(!row0_fields.contains_key(""));
    }

    #[test]
    fn test_xlsx_empty_file() {
        let xlsx_data = create_empty_xlsx();
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);

        assert!(matches!(result, FileParsingOutcome::Rejected { .. }));
    }

    #[test]
    fn test_xlsx_header_only() {
        let xlsx_data = create_header_only_xlsx(&["name", "age", "city"]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);

        assert_eq!(parsed_rows(&result).len(), 0);
        assert!(rejected_rows(&result).is_empty());
    }

    #[test]
    fn test_xlsx_missing_sheet_by_name() {
        let xlsx_data = create_simple_xlsx(&["name"], &[vec!["John"]]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByName("NonExistent".to_string()),
        };
        let result = config.parse(&xlsx_data);

        assert!(matches!(result, FileParsingOutcome::Rejected { .. }));
    }

    #[test]
    fn test_xlsx_sheet_by_name() {
        let xlsx_data = create_multi_sheet_xlsx(
            "Summary",
            &[vec!["Total", "100"]],
            "Data",
            &[vec!["name", "value"], vec!["Item1", "50"]],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByName("Data".to_string()),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("name")
                .map(|field| &field.value),
            Some(&Some("Item1".to_string()))
        );
    }

    #[test]
    fn test_xlsx_sheet_by_index() {
        let xlsx_data = create_multi_sheet_xlsx(
            "First",
            &[vec!["a", "b"], vec!["1", "2"]],
            "Second",
            &[vec!["x", "y"], vec!["10", "20"]],
        );
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(1),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("x")
                .map(|field| &field.value),
            Some(&Some("10".to_string()))
        );
    }

    #[test]
    fn test_xlsx_invalid_header_row_index() {
        let xlsx_data = create_simple_xlsx(&["name"], &[vec!["John"]]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 100,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);

        assert!(matches!(result, FileParsingOutcome::Rejected { .. }));
    }

    #[test]
    fn test_xlsx_line_number_correctness() {
        let xlsx_data = create_simple_xlsx(&["name"], &[vec!["A"], vec!["B"], vec!["C"]]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert!(has_line(&rows, 2));
        assert!(has_line(&rows, 3));
        assert!(has_line(&rows, 4));
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_xlsx_source_tracking() {
        let xlsx_data = create_simple_xlsx(&["name"], &[vec!["John"]]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.get(0).unwrap().source,
            SourceRecord::FileRow { line_number: 2 }
        );
    }

    #[test]
    fn test_xlsx_fewer_columns_than_header() {
        let xlsx_data = create_simple_xlsx(&["name", "age", "city"], &[vec!["John"]]);
        let config = XlsxParsingConfigV1 {
            header_row_index: 0,
            sheet_selection: SheetSelection::ByIndex(0),
        };
        let result = config.parse(&xlsx_data);
        let rows = parsed_rows(&result);

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("name")
                .map(|field| &field.value),
            Some(&Some("John".to_string()))
        );
        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("age")
                .map(|field| &field.value),
            Some(&None)
        );
        assert_eq!(
            rows.get(0)
                .unwrap()
                .fields
                .get("city")
                .map(|field| &field.value),
            Some(&None)
        );
    }
}
