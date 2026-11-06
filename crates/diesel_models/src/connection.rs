use std::future::Future;

use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use diesel::result::Error as DieselError;

use crate::PgPooledConn;

/// A connection that read queries can run on.
pub trait ReadConnection: Send + Sync {
    fn async_conn(&self) -> &PgPooledConn;
}

/// A connection that write queries can run on. Every write connection can also read.
#[async_trait]
pub trait WriteConnection: ReadConnection {
    /// Runs `f` in a database transaction, giving it the transaction's write connection.
    /// Commits if `f` returns `Ok`, rolls back if it returns `Err`.
    async fn transaction<R, E, F, Fut>(&self, f: F) -> Result<R, E>
    where
        R: Send + 'static,
        E: From<DieselError> + Send + 'static,
        F: FnOnce(TransactionConnection) -> Fut + Send,
        Fut: Future<Output = Result<R, E>> + Send,
    {
        self.async_conn()
            .transaction_async(|conn| f(TransactionConnection(conn)))
            .await
    }
}

/// The connection a database transaction runs on. Only `WriteConnection::transaction` creates one.
pub struct TransactionConnection(PgPooledConn);

impl ReadConnection for TransactionConnection {
    fn async_conn(&self) -> &PgPooledConn {
        &self.0
    }
}

impl WriteConnection for TransactionConnection {}
