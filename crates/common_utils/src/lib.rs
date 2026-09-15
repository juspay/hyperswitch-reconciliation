pub mod auth;
pub mod chunking;
pub mod consts;
pub mod crypto;
pub mod custom_serde;
pub mod date_time;
pub mod encryption;
pub mod errors;
pub mod ext_traits;
pub mod fp_utils;
pub mod id_generation;
pub mod id_type;
pub mod keymanager;
pub mod logging;
pub mod macros;
pub mod pii;
pub mod request;
pub mod transformers;
pub mod types;
pub mod validation;

pub use date_time::CommonPrimitiveDateTime as PrimitiveDateTime;
pub use id_type::GenerateId;

/// Generates a reference ID with default length using the given prefix
fn generate_ref_id_with_default_length<const MAX_LENGTH: u8, const MIN_LENGTH: u8>(
    prefix: &str,
) -> id_type::LengthId<MAX_LENGTH, MIN_LENGTH> {
    id_type::LengthId::<MAX_LENGTH, MIN_LENGTH>::new(prefix)
}
