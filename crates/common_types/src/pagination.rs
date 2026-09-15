use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CursorPagination<S> {
    pub limit: Option<u16>,
    #[serde(default)]
    pub direction: CursorDirection,
    pub sort_by: S,
    #[serde(default)]
    pub order: SortOrder,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

#[derive(Clone, Debug, Copy, ToSchema, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CursorDirection {
    #[default]
    Next,
    Previous,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CursorPage<T, C = String> {
    pub items: Vec<T>,
    pub next_cursor: Option<C>,
    pub prev_cursor: Option<C>,
}

pub struct PaginationQuery<S> {
    pub direction: CursorDirection,
    pub sort_by: S,
    pub order: SortOrder,
    pub fetch_limit: u16,
}

impl<S> CursorPagination<S> {
    pub fn request_limit(&self, default_limit: u16, max_limit: u16) -> u16 {
        self.limit.unwrap_or(default_limit).clamp(1, max_limit)
    }

    pub fn into_query(self, default_limit: u16, max_limit: u16) -> PaginationQuery<S> {
        let fetch_limit = self.request_limit(default_limit, max_limit);
        PaginationQuery {
            direction: self.direction,
            sort_by: self.sort_by,
            order: self.order,
            fetch_limit,
        }
    }

    pub fn build_cursors<T>(&self, items: &[T], has_more: bool) -> (Option<S>, Option<S>)
    where
        S: CursorSort<T>,
    {
        let first_cursor = items.first().map(|item| self.sort_by.cursor_at(item));
        let last_cursor = items.last().map(|item| self.sort_by.cursor_at(item));
        let cursor_supplied = self.sort_by.cursor_supplied();

        match self.direction {
            CursorDirection::Next => (
                if has_more { last_cursor } else { None },
                if cursor_supplied { first_cursor } else { None },
            ),
            CursorDirection::Previous => (
                if cursor_supplied { last_cursor } else { None },
                if has_more { first_cursor } else { None },
            ),
        }
    }
}

impl<S> PaginationQuery<S> {
    pub fn ascending(&self) -> bool {
        match (self.order, self.direction) {
            (SortOrder::Asc, CursorDirection::Next)
            | (SortOrder::Desc, CursorDirection::Previous) => true,
            (SortOrder::Asc, CursorDirection::Previous)
            | (SortOrder::Desc, CursorDirection::Next) => false,
        }
    }
}

pub trait CursorSort<T>: Sized {
    fn cursor_at(&self, item: &T) -> Self;
    fn cursor_supplied(&self) -> bool;
}
