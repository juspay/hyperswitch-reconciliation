use common_types::metadata_schema::MetadataSchemaData;
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel::{query_builder::AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::metadata_schema;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = metadata_schema, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct MetadataSchema {
    pub id: String,
    pub schema_id: String,
    pub profile_id: String,
    pub account_id: String,
    pub version: i32,
    pub schema_data: MetadataSchemaData,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = metadata_schema, check_for_backend(diesel::pg::Pg))]
pub struct MetadataSchemaNew {
    pub id: String,
    pub schema_id: String,
    pub profile_id: String,
    pub account_id: String,
    pub version: i32,
    pub schema_data: MetadataSchemaData,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

impl MetadataSchemaNew {
    pub fn new(profile_id: String, account_id: String, schema_data: MetadataSchemaData) -> Self {
        let now = date_time::now();
        Self {
            id: generate_id_with_default_len("meta_sch"),
            schema_id: generate_id_with_default_len("meta_sch_main"),
            profile_id,
            account_id,
            version: 1,
            schema_data,
            created_at: now,
            last_modified_at: now,
        }
    }

    pub fn new_versioned(previous: MetadataSchema, update: MetadataSchemaVersionUpdate) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("meta_sch");

        let schema_data = match update {
            MetadataSchemaVersionUpdate::SchemaUpdate { schema_data } => schema_data,
        };

        Self {
            id,
            schema_id: previous.schema_id,
            profile_id: previous.profile_id,
            account_id: previous.account_id,
            version: previous.version + 1,
            schema_data,
            created_at: now,
            last_modified_at: now,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MetadataSchemaVersionUpdate {
    SchemaUpdate { schema_data: MetadataSchemaData },
}

// In-place update
#[derive(Clone, Debug)]
pub enum MetadataSchemaUpdate {
    SchemaUpdate { schema_data: MetadataSchemaData },
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = metadata_schema)]
pub struct MetadataSchemaUpdateInternal {
    pub schema_data: Option<MetadataSchemaData>,
    pub last_modified_at: PrimitiveDateTime,
}

impl From<MetadataSchemaUpdate> for MetadataSchemaUpdateInternal {
    fn from(value: MetadataSchemaUpdate) -> Self {
        match value {
            MetadataSchemaUpdate::SchemaUpdate { schema_data } => {
                let now = date_time::now();

                Self {
                    schema_data: Some(schema_data),
                    last_modified_at: now,
                }
            }
        }
    }
}
