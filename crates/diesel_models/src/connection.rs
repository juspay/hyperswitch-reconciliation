use std::future::Future;

use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use diesel::{result::Error as DieselError, PgConnection};

use crate::PgPooledConn;

/// A connection checked out of the pool. It goes back to the pool when dropped.
pub type PooledConn<'a> =
    bb8::PooledConnection<'a, async_bb8_diesel::ConnectionManager<PgConnection>>;

/// A connection that read queries can run on.
pub trait ReadConnection: Send + Sync {
    fn inner(&self) -> &PgPooledConn;
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
        F: FnOnce(WriteConn<'static>) -> Fut + Send,
        Fut: Future<Output = Result<R, E>> + Send,
    {
        self.inner()
            .transaction_async(|conn| f(WriteConn(WriteInner::Owned(conn))))
            .await
    }
}

/// A connection for read queries only.
pub struct ReadConn<'a>(PooledConn<'a>);

impl<'a> ReadConn<'a> {
    pub fn new(conn: PooledConn<'a>) -> Self {
        Self(conn)
    }
}

/// A connection for write queries. It can also read.
pub struct WriteConn<'a>(WriteInner<'a>);

// `Pooled` holds a guard borrowed from the pool. `Owned` holds the connection `transaction`
// hands its closure by value.
enum WriteInner<'a> {
    Pooled(PooledConn<'a>),
    Owned(PgPooledConn),
}

impl<'a> WriteConn<'a> {
    pub fn new(conn: PooledConn<'a>) -> Self {
        Self(WriteInner::Pooled(conn))
    }
}

impl ReadConnection for ReadConn<'_> {
    fn inner(&self) -> &PgPooledConn {
        &self.0
    }
}

impl ReadConnection for WriteConn<'_> {
    fn inner(&self) -> &PgPooledConn {
        match &self.0 {
            WriteInner::Pooled(conn) => conn,
            WriteInner::Owned(conn) => conn,
        }
    }
}

impl WriteConnection for WriteConn<'_> {}
