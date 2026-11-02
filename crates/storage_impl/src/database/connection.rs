use async_bb8_diesel::AsyncConnection;
use diesel::PgConnection;
use diesel_models::{PgPooledConn as AsyncPgConn, ReadConn, WriteConn};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn<'a> =
    bb8::PooledConnection<'a, async_bb8_diesel::ConnectionManager<PgConnection>>;

/// Handed out by `DatabaseRead`; only read queries accept it.
pub struct ReadConnection<'a>(pub(crate) PgPooledConn<'a>);

pub struct WriteConnection<'a>(pub(crate) PgPooledConn<'a>);

impl ReadConn for ReadConnection<'_> {
    fn as_read_conn(&self) -> &AsyncPgConn {
        self.0.as_async_conn()
    }
}

impl ReadConn for WriteConnection<'_> {
    fn as_read_conn(&self) -> &AsyncPgConn {
        self.0.as_async_conn()
    }
}

impl WriteConn for WriteConnection<'_> {
    fn as_write_conn(&self) -> &AsyncPgConn {
        self.0.as_async_conn()
    }
}
