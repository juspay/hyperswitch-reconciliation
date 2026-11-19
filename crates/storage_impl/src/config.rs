use masking::{PeekInterface, Secret};

/// Connection settings for a Postgres database. Binaries build this from their own config.
#[derive(Debug, Clone)]
pub struct Database {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
}

impl Database {
    pub fn get_database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.peek(),
            self.host,
            self.port,
            self.dbname
        )
    }
}
