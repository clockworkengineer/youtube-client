//! Unit tests for Page container and pagination state transitions

use youtube_client_lib::models::Page;

#[test]
fn test_page_lifecycle() {
    let empty_page: Page<String> = Page::new(Vec::new(), None);
    assert!(empty_page.is_empty());
    assert_eq!(empty_page.len(), 0);
    assert!(!empty_page.has_more());

    let page1 = Page::new(
        vec!["item1".to_string(), "item2".to_string()],
        Some("token_123".to_string()),
    );
    assert!(!page1.is_empty());
    assert_eq!(page1.len(), 2);
    assert!(page1.has_more());
    assert_eq!(page1.next_page_token.as_deref(), Some("token_123"));

    let page2 = Page::new(vec!["item3".to_string()], None);
    assert_eq!(page2.len(), 1);
    assert!(!page2.has_more());
}
