//! # Generic Pagination Container

/// A single page of items returned by a paginated API query.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Page<T> {
    /// The items contained in the current page.
    pub items: Vec<T>,
    /// Opaque token used to request the subsequent page, or `None` if this is the last page.
    pub next_page_token: Option<String>,
}

impl<T> Page<T> {
    /// Create a new `Page` from items and an optional continuation token.
    pub fn new(items: Vec<T>, next_page_token: Option<String>) -> Self {
        Self {
            items,
            next_page_token,
        }
    }

    /// Check if there are more pages available after this one.
    pub fn has_more(&self) -> bool {
        self.next_page_token.is_some()
    }

    /// Number of items on the current page.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check whether the current page contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
