# Pattern: Database Query

## When to Use
- Building queries with optional filters
- Implementing pagination (offset for small lists, cursor/keyset for large or hot lists)
- Creating complex queries with joins
- Using transactions for atomic operations
- Converting between API models and database models

## Quick Example
```rust
// Query with optional filter
pub async fn list_accounts(
    conn: &PgPooledConn,
    profile_id: &str,
    is_active: Option<bool>,
) -> StorageResult<Vec<Account>> {
    let mut query = dsl::account
        .filter(dsl::profile_id.eq(profile_id))
        .into_boxed();
    
    if let Some(active) = is_active {
        query = query.filter(dsl::is_active.eq(active));
    }
    
    query.order(dsl::created_at.desc())
        .get_results_async(conn)
        .await
}
```

## Full Implementation
```rust
// 1. Define database models
use diesel::{Queryable, Selectable, Insertable, AsChangeset};
use diesel_models::schema::account;

// Query model (retrieved from DB)
#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = account)]
pub struct Account {
    pub account_id: String,
    pub account_name: String,
    pub account_type: BalanceDirection,
    pub initial_balance: MinorUnit,
    pub posted_debits: MinorUnit,
    pub posted_credits: MinorUnit,
    pub pending_debits: MinorUnit,
    pub pending_credits: MinorUnit,
    pub currency: Currency,
    pub profile_id: String,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}

// Insert model (for creating)
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = account)]
pub struct AccountNew {
    pub account_id: String,
    pub account_name: String,
    pub account_type: BalanceDirection,
    pub initial_balance: MinorUnit,
    pub posted_debits: MinorUnit,
    pub posted_credits: MinorUnit,
    pub pending_debits: MinorUnit,
    pub pending_credits: MinorUnit,
    pub currency: Currency,
    pub profile_id: String,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}

// Update model (for updates)
#[derive(AsChangeset)]
#[diesel(table_name = account)]
pub struct AccountUpdate {
    pub account_name: Option<String>,
    pub last_modified_at: time::PrimitiveDateTime,
}

// 2. Basic CRUD operations
impl Account {
    // INSERT
    pub async fn insert(
        conn: &PgPooledConn,
        new_account: AccountNew,
    ) -> StorageResult<Self> {
        diesel::insert_into(account::table)
            .values(&new_account)
            .get_result_async(conn)
            .await
            .map_err(Into::into)
    }
    
    // SELECT by ID
    pub async fn find_by_id(
        conn: &PgPooledConn,
        account_id: &str,
    ) -> StorageResult<Self> {
        account::table
            .find(account_id)
            .first_async(conn)
            .await
            .map_err(Into::into)
    }
    
    // UPDATE
    pub async fn update(
        conn: &PgPooledConn,
        account_id: &str,
        update: AccountUpdate,
    ) -> StorageResult<Self> {
        diesel::update(account::table.find(account_id))
            .set(&update)
            .get_result_async(conn)
            .await
            .map_err(Into::into)
    }
}

// 3. Complex queries with filters
pub async fn list_accounts_with_filters(
    conn: &PgPooledConn,
    profile_id: &str,
    account_type: Option<BalanceDirection>,
    currency: Option<Currency>,
    min_balance: Option<MinorUnit>,
) -> StorageResult<Vec<Account>> {
    use diesel::prelude::*;
    use account::dsl;
    
    // Start with base query
    let mut query = dsl::account
        .filter(dsl::profile_id.eq(profile_id))
        .into_boxed();
    
    // Apply optional filters
    if let Some(acc_type) = account_type {
        query = query.filter(dsl::account_type.eq(acc_type));
    }
    
    if let Some(curr) = currency {
        query = query.filter(dsl::currency.eq(curr));
    }
    
    if let Some(min_bal) = min_balance {
        query = query.filter(dsl::posted_balance.ge(min_bal));
    }
    
    // Order and execute
    query
        .order(dsl::created_at.desc())
        .get_results_async(conn)
        .await
        .map_err(Into::into)
}

// 3a. Filtering with multiple values (e.g., from query params)
pub async fn list_entries_with_status_filter(
    conn: &PgPooledConn,
    account_id: &str,
    status: Option<Vec<EntryStatus>>,
) -> StorageResult<Vec<Entry>> {
    use diesel::prelude::*;
    use entries::dsl;
    
    let mut query = dsl::entries
        .filter(dsl::account_id.eq(account_id))
        .into_boxed();
    
    // Filter by multiple values using eq_any
    if let Some(status_list) = status {
        query = query.filter(dsl::status.eq_any(status_list));
    }
    
    query
        .order(dsl::created_at.desc())
        .get_results_async(conn)
        .await
        .map_err(Into::into)
}

// 4. Pagination support
pub async fn list_accounts_paginated(
    conn: &PgPooledConn,
    profile_id: &str,
    limit: i64,
    offset: i64,
) -> StorageResult<(Vec<Account>, i64)> {
    use diesel::dsl::count_star;
    
    // Get total count
    let total = account::table
        .filter(account::profile_id.eq(profile_id))
        .select(count_star())
        .first_async::<i64>(conn)
        .await?;
    
    // Get paginated results
    let accounts = account::table
        .filter(account::profile_id.eq(profile_id))
        .order(account::created_at.desc())
        .limit(limit)
        .offset(offset)
        .get_results_async(conn)
        .await?;
    
    Ok((accounts, total))
}

// 5. Joins between tables
pub async fn get_account_with_entries(
    conn: &PgPooledConn,
    account_id: &str,
) -> StorageResult<Vec<(Account, Entry)>> {
    use diesel_models::schema::entries;
    
    account::table
        .inner_join(entries::table.on(
            account::account_id.eq(entries::account_id)
        ))
        .filter(account::account_id.eq(account_id))
        .filter(entries::status.eq(EntryStatus::Posted))
        .order(entries::created_at.desc())
        .select((account::all_columns, entries::all_columns))
        .load_async(conn)
        .await
        .map_err(Into::into)
}

// 6. Transactions for atomic operations
pub async fn transfer_between_accounts(
    conn: &mut PgPooledConn,
    from_id: &str,
    to_id: &str,
    amount: MinorUnit,
) -> StorageResult<()> {
    use async_bb8_diesel::AsyncConnection;
    
    conn.transaction(|conn| async move {
        // Debit from account
        diesel::update(account::table.find(from_id))
            .set((
                account::posted_balance.eq(account::posted_balance - amount),
                account::last_modified_at.eq(common_utils::date_time::now()),
            ))
            .execute_async(conn)
            .await?;
        
        // Credit to account
        diesel::update(account::table.find(to_id))
            .set((
                account::posted_balance.eq(account::posted_balance + amount),
                account::last_modified_at.eq(common_utils::date_time::now()),
            ))
            .execute_async(conn)
            .await?;
        
        Ok::<_, diesel::result::Error>(())
    })
    .await
    .map_err(Into::into)
}

// 7. Complex query with subqueries
pub async fn find_accounts_with_recent_activity(
    conn: &PgPooledConn,
    profile_id: &str,
    days: i32,
) -> StorageResult<Vec<Account>> {
    use diesel::dsl::{exists, select};
    use diesel_models::schema::entries;
    
    let cutoff_date = common_utils::date_time::now() - chrono::Duration::days(days as i64);
    
    account::table
        .filter(account::profile_id.eq(profile_id))
        .filter(exists(
            entries::table
                .filter(entries::account_id.eq(account::account_id))
                .filter(entries::created_at.gt(cutoff_date))
        ))
        .order(account::last_modified_at.desc())
        .get_results_async(conn)
        .await
        .map_err(Into::into)
}

// 8. Bulk operations
pub async fn bulk_update_account_status(
    conn: &PgPooledConn,
    account_ids: Vec<String>,
    is_active: bool,
) -> StorageResult<usize> {
    diesel::update(account::table)
        .filter(account::account_id.eq_any(account_ids))
        .set((
            account::is_active.eq(is_active),
            account::last_modified_at.eq(common_utils::date_time::now()),
        ))
        .execute_async(conn)
        .await
        .map_err(Into::into)
}

// 9. Using in Store implementation
#[async_trait::async_trait]
impl AccountInterface for Store {
    async fn find_account_by_id(
        &self,
        account_id: &str,
    ) -> CustomResult<Account, DatabaseError> {
        let mut conn = self.get_conn().await?;
        Account::find_by_id(&mut conn, account_id).await
    }
    
    async fn list_accounts_for_profile(
        &self,
        profile_id: &str,
        filters: AccountFilters,
    ) -> CustomResult<Vec<Account>, DatabaseError> {
        let mut conn = self.get_conn().await?;
        list_accounts_with_filters(
            &mut conn,
            profile_id,
            filters.account_type,
            filters.currency,
            filters.min_balance,
        )
        .await
    }
}
```

## Key Points
- **Always use async methods** - `get_results_async`, not `get_results`
- **Connection handling** - Get connection once, reuse for query
- **Optional filters** - Use `into_boxed()` for dynamic queries
- **Transactions** - Use connection.transaction() for atomicity
- **Timestamps** - Always update `last_modified_at`
- **Error conversion** - Use `.map_err(Into::into)` for StorageResult

## Common Mistakes
- Holding connections across await points
- Using blocking diesel calls instead of async
- Not updating last_modified_at timestamp
- Missing indexes on frequently queried fields
- Not handling DatabaseError::NotFound appropriately
- Using unwrap() on query results

## Related Patterns
- [ERROR-HANDLING](ERROR-HANDLING.md) - Converting database errors
- [HANDLE-MONEY](HANDLE-MONEY.md) - Storing monetary values
- [ADD-ENDPOINT](ADD-ENDPOINT.md) - Using queries in endpoints

## Gotchas to Remember
- Enums must be created before tables in migrations
- Use BIGINT for all monetary amounts
- Foreign keys aren't enforced at DB level
- Soft deletes use discarded_at timestamp
- Array fields need DieselArray wrapper
- Profile-scoped queries for multi-tenancy
- Cursor/keyset lists over-fetch `fetch_limit + 1` in the query (NOT in `into_query`) to compute `has_more`, then truncate
- Cursor seeks need a composite `(sort_col, id)` key + matching `ORDER BY` — the `id` tie-breaker stops equal-value rows skipping/duplicating across pages
- A cursor's `sort_col` must serialize at the DB column's actual precision — `PrimitiveDateTime` cursor fields need `custom_serde::iso8601_micros` (matches Postgres `Timestamp` microseconds), not the millisecond `iso8601` used elsewhere, or the `.eq()` tie-break silently mismatches
- `CursorDirection::Previous` queries the index reversed; reverse the page in memory before returning so callers always get the requested order
- New sortable entity? Implement both `CursorSort::cursor_at` and `cursor_supplied`, then emit cursors via `CursorPagination::build_cursors`
