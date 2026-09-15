use common_types::transformation::{ParsingConfig, ParsingConfigV1};

use crate::domain::metadata_schema::types::FileParsingOutcome;

pub mod csv_parsing;
pub mod fixed_width_parsing;
pub mod xlsx_parsing;

// TODO: Handle fully blank lines consistently in all parsers

pub trait FileParser {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome;
}

impl FileParser for ParsingConfig {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        match self {
            Self::V1(parsing_config_v1) => parsing_config_v1.parse(input),
        }
    }
}

impl FileParser for ParsingConfigV1 {
    fn parse(&self, input: &[u8]) -> FileParsingOutcome {
        match self {
            Self::Csv(csv_parsing_config) => csv_parsing_config.parse(input),
            Self::Xlsx(xlsx_parsing_config) => xlsx_parsing_config.parse(input),
            Self::FixedWidth(fixed_width_config) => fixed_width_config.parse(input),
        }
    }
}
