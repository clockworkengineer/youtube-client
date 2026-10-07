//! Unit tests for Page container and pagination state transitions

use futures_util::StreamExt;
use youtube_client_lib::models::{Page, Subscription, Video};
use youtube_client_lib::{MockYoutubeClient, YoutubeBackend};

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

#[tokio::test]
async fn test_streaming_pagination_backend() {
    let mock = MockYoutubeClient::new();
    {
        let mut subs = mock.subscriptions.lock().unwrap();
        for i in 1..=5 {
            subs.push(Subscription {
                id: format!("sub_{i}"),
                title: format!("Subscription {i}"),
                description: String::new(),
                channel_id: format!("UC_chan_{i}"),
                thumbnail_url: String::new(),
            });
        }
    }

    let backend = YoutubeBackend::mock(mock);
    let mut stream = backend.stream_subscriptions(2);

    let mut collected = Vec::new();
    while let Some(item_res) = stream.next().await {
        let item = item_res.expect("Stream yield failed");
        collected.push(item);
    }

    assert_eq!(collected.len(), 5);
    assert_eq!(collected[0].title, "Subscription 1");
    assert_eq!(collected[4].title, "Subscription 5");
}

#[tokio::test]
async fn test_streaming_videos_backend() {
    let mock = MockYoutubeClient::new();
    {
        let mut vids = mock.videos_by_channel.lock().unwrap();
        let list = vec![
            Video {
                id: "v1".to_string(),
                title: "Video 1".to_string(),
                description: String::new(),
                published_at: String::new(),
                channel_title: String::new(),
                thumbnail_url: String::new(),
            },
            Video {
                id: "v2".to_string(),
                title: "Video 2".to_string(),
                description: String::new(),
                published_at: String::new(),
                channel_title: String::new(),
                thumbnail_url: String::new(),
            },
        ];
        vids.insert("UC_test".to_string(), list);
    }

    let backend = YoutubeBackend::mock(mock);
    let mut stream = backend.stream_videos("UC_test", 10);

    let mut titles = Vec::new();
    while let Some(v_res) = stream.next().await {
        titles.push(v_res.unwrap().title);
    }

    assert_eq!(titles, vec!["Video 1", "Video 2"]);
}
