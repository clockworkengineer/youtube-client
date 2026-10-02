//! # CLI Output Formatting Engine
//!
//! Provides decoupled output formatting strategies for CLI results, adhering to the
//! Open/Closed and Single Responsibility principles.

use youtube_client_lib::models::{
    ChannelDetails, Comment, Page, Playlist, Subscription, Video, VideoDetails,
};
use youtube_client_lib::utils::{format_table, truncate};

/// Trait defining the contract for formatting domain types to output strings.
pub trait OutputFormatter<T> {
    /// Format a single item into a displayable string.
    fn format_item(&self, item: &T) -> String;

    /// Format a list of items into a displayable string.
    fn format_list(&self, items: &[T]) -> String;

    /// Format a paginated collection of items, including resumption tokens.
    fn format_page(&self, page: &Page<T>) -> String {
        let mut out = self.format_list(&page.items);
        if let Some(ref next) = page.next_page_token {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&format!(
                "Next page token: {next}\nFetch next page with: --page-token {next}"
            ));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// JSON Formatter
// ---------------------------------------------------------------------------

/// Formatter that serializes models into pretty-printed JSON.
pub struct JsonFormatter;

impl<T: serde::Serialize> OutputFormatter<T> for JsonFormatter {
    fn format_item(&self, item: &T) -> String {
        serde_json::to_string_pretty(item).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
    }

    fn format_list(&self, items: &[T]) -> String {
        serde_json::to_string_pretty(items).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
    }

    fn format_page(&self, page: &Page<T>) -> String {
        serde_json::to_string_pretty(page).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
    }
}

// ---------------------------------------------------------------------------
// CSV Formatter
// ---------------------------------------------------------------------------

/// Formatter that outputs tabular data in standard comma-separated values (CSV).
pub struct CsvFormatter;

fn escape_csv(val: &str) -> String {
    if val.contains(',') || val.contains('"') || val.contains('\n') || val.contains('\r') {
        format!("\"{}\"", val.replace('"', "\"\""))
    } else {
        val.to_string()
    }
}

impl OutputFormatter<Video> for CsvFormatter {
    fn format_item(&self, item: &Video) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Video]) -> String {
        let mut lines = vec!["Index,Title,Video ID,Published At,Channel".to_string()];
        for (idx, vid) in items.iter().enumerate() {
            lines.push(format!(
                "{},{},{},{},{}",
                idx + 1,
                escape_csv(&vid.title),
                escape_csv(&vid.id),
                escape_csv(&vid.published_at),
                escape_csv(&vid.channel_title)
            ));
        }
        lines.join("\n")
    }
}

impl OutputFormatter<Subscription> for CsvFormatter {
    fn format_item(&self, item: &Subscription) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Subscription]) -> String {
        let mut lines = vec!["Index,Title,Channel ID".to_string()];
        for (idx, sub) in items.iter().enumerate() {
            lines.push(format!(
                "{},{},{}",
                idx + 1,
                escape_csv(&sub.title),
                escape_csv(&sub.channel_id)
            ));
        }
        lines.join("\n")
    }
}

impl OutputFormatter<Playlist> for CsvFormatter {
    fn format_item(&self, item: &Playlist) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Playlist]) -> String {
        let mut lines = vec!["Index,Title,Playlist ID,Item Count".to_string()];
        for (idx, pl) in items.iter().enumerate() {
            lines.push(format!(
                "{},{},{},{}",
                idx + 1,
                escape_csv(&pl.title),
                escape_csv(&pl.id),
                pl.video_count
            ));
        }
        lines.join("\n")
    }
}

impl OutputFormatter<Comment> for CsvFormatter {
    fn format_item(&self, item: &Comment) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Comment]) -> String {
        let mut lines = vec!["Author,Comment,Likes,Published At".to_string()];
        for c in items {
            lines.push(format!(
                "{},{},{},{}",
                escape_csv(&c.author_name),
                escape_csv(&c.text_display),
                c.like_count,
                escape_csv(&c.published_at)
            ));
        }
        lines.join("\n")
    }
}

// ---------------------------------------------------------------------------
// Table / Human-Readable Formatter
// ---------------------------------------------------------------------------

/// Formatter that outputs aligned, styled terminal tables and cards.
pub struct TableFormatter;

impl OutputFormatter<Video> for TableFormatter {
    fn format_item(&self, item: &Video) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Video]) -> String {
        format_table(
            &["Index", "Title", "Video ID", "Published At"],
            &[5, 40, 15, 15],
            items,
            |vid, idx| {
                vec![
                    (idx + 1).to_string(),
                    truncate(&vid.title, 38).into_owned(),
                    vid.id.clone(),
                    truncate(&vid.published_at, 10).into_owned(),
                ]
            },
        )
    }
}

impl OutputFormatter<Subscription> for TableFormatter {
    fn format_item(&self, item: &Subscription) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Subscription]) -> String {
        format_table(
            &["Index", "Title", "Channel ID"],
            &[5, 35, 30],
            items,
            |sub, idx| {
                vec![
                    (idx + 1).to_string(),
                    truncate(&sub.title, 33).into_owned(),
                    sub.channel_id.clone(),
                ]
            },
        )
    }
}

impl OutputFormatter<Playlist> for TableFormatter {
    fn format_item(&self, item: &Playlist) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Playlist]) -> String {
        format_table(
            &["Index", "Title", "Playlist ID", "Item Count"],
            &[5, 40, 30, 12],
            items,
            |pl, idx| {
                vec![
                    (idx + 1).to_string(),
                    truncate(&pl.title, 38).into_owned(),
                    pl.id.clone(),
                    pl.video_count.to_string(),
                ]
            },
        )
    }
}

impl OutputFormatter<Comment> for TableFormatter {
    fn format_item(&self, item: &Comment) -> String {
        self.format_list(std::slice::from_ref(item))
    }

    fn format_list(&self, items: &[Comment]) -> String {
        if items.is_empty() {
            return "No comments found.".to_string();
        }
        format_table(
            &["Author", "Comment", "Likes", "Published At"],
            &[20, 50, 8, 12],
            items,
            |c, _idx| {
                vec![
                    truncate(&c.author_name, 18).into_owned(),
                    truncate(&c.text_display.replace('\n', " "), 48).into_owned(),
                    c.like_count.to_string(),
                    truncate(&c.published_at, 10).into_owned(),
                ]
            },
        )
    }
}

impl OutputFormatter<VideoDetails> for TableFormatter {
    fn format_item(&self, details: &VideoDetails) -> String {
        let mut out = String::new();
        out.push_str("==================================================\n");
        out.push_str(&format!("  🎬 {}\n", details.title));
        out.push_str("==================================================\n");
        out.push_str(&format!("  Video ID:        {}\n", details.id));
        out.push_str(&format!(
            "  Channel:         {} ({})\n",
            details.channel_title, details.channel_id
        ));
        out.push_str(&format!("  Published:       {}\n", details.published_at));
        out.push_str(&format!(
            "  Duration:        {} ({} seconds)\n",
            details.duration_formatted, details.duration_seconds
        ));
        out.push_str(&format!("  Views:           {}\n", details.view_count));
        out.push_str(&format!("  Likes:           {}\n", details.like_count));
        if let Some(dislikes) = details.dislike_count {
            out.push_str(&format!("  Dislikes (RYD):  {dislikes}\n"));
        }
        out.push_str(&format!("  Comments:        {}\n", details.comment_count));
        if !details.tags.is_empty() {
            out.push_str(&format!("  Tags:            {}\n", details.tags.join(", ")));
        }
        out.push_str("--------------------------------------------------\n");
        out.push_str(&format!("Description:\n{}\n", details.description));
        out.push_str("==================================================");
        out
    }

    fn format_list(&self, items: &[VideoDetails]) -> String {
        items
            .iter()
            .map(|item| self.format_item(item))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl OutputFormatter<ChannelDetails> for TableFormatter {
    fn format_item(&self, details: &ChannelDetails) -> String {
        let mut out = String::new();
        out.push_str("==================================================\n");
        out.push_str(&format!("  📺 {}\n", details.title));
        if let Some(ref handle) = details.custom_url {
            out.push_str(&format!("  Handle:          {handle}\n"));
        }
        out.push_str("==================================================\n");
        out.push_str(&format!("  Channel ID:      {}\n", details.id));
        out.push_str(&format!(
            "  Subscribers:     {}\n",
            details.subscriber_count
        ));
        out.push_str(&format!("  Total Videos:    {}\n", details.video_count));
        out.push_str(&format!("  Lifetime Views:  {}\n", details.view_count));
        if !details.description.is_empty() {
            out.push_str("--------------------------------------------------\n");
            out.push_str(&format!("About:\n{}\n", details.description));
        }
        out.push_str("==================================================");
        out
    }

    fn format_list(&self, items: &[ChannelDetails]) -> String {
        items
            .iter()
            .map(|item| self.format_item(item))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

// ---------------------------------------------------------------------------
// Selector Helper
// ---------------------------------------------------------------------------

/// Select an appropriate formatter based on the `json` boolean flag.
pub fn select_formatter<T: 'static>(json: bool) -> Box<dyn OutputFormatter<T>>
where
    TableFormatter: OutputFormatter<T>,
    JsonFormatter: OutputFormatter<T>,
{
    if json {
        Box::new(JsonFormatter)
    } else {
        Box::new(TableFormatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_formatter() {
        let video = Video {
            id: "vid_1".to_string(),
            title: "Test Video".to_string(),
            description: "Desc".to_string(),
            published_at: "2026-01-01T00:00:00Z".to_string(),
            thumbnail_url: "https://example.com/thumb.jpg".to_string(),
            channel_title: "Rust Channel".to_string(),
        };

        let formatter = JsonFormatter;
        let json = formatter.format_item(&video);
        assert!(json.contains("\"id\": \"vid_1\""));
        assert!(json.contains("\"title\": \"Test Video\""));
    }

    #[test]
    fn test_csv_formatter() {
        let video = Video {
            id: "vid_1".to_string(),
            title: "Test, with comma".to_string(),
            description: "Desc".to_string(),
            published_at: "2026-01-01T00:00:00Z".to_string(),
            thumbnail_url: "https://example.com/thumb.jpg".to_string(),
            channel_title: "Rust Channel".to_string(),
        };

        let formatter = CsvFormatter;
        let csv = formatter.format_list(&[video]);
        assert!(csv.contains("Index,Title,Video ID,Published At,Channel"));
        assert!(csv.contains("\"Test, with comma\""));
    }

    #[test]
    fn test_table_formatter_video_page() {
        let page = Page::new(
            vec![Video {
                id: "vid_1".to_string(),
                title: "Test Video".to_string(),
                description: "Desc".to_string(),
                published_at: "2026-01-01T00:00:00Z".to_string(),
                thumbnail_url: "https://example.com/thumb.jpg".to_string(),
                channel_title: "Rust Channel".to_string(),
            }],
            Some("NEXT_PAGE".to_string()),
        );

        let formatter = select_formatter::<Video>(false);
        let output = formatter.format_page(&page);
        assert!(output.contains("Test Video"));
        assert!(output.contains("vid_1"));
        assert!(output.contains("Next page token: NEXT_PAGE"));
        assert!(output.contains("--page-token NEXT_PAGE"));
    }

    #[test]
    fn test_table_formatter_details() {
        let details = VideoDetails {
            id: "vid_123".to_string(),
            title: "Deep Dive into Rust".to_string(),
            description: "A great video".to_string(),
            published_at: "2026-01-01".to_string(),
            channel_id: "chan_456".to_string(),
            channel_title: "Rustaceans".to_string(),
            thumbnail_url: "http://thumb.jpg".to_string(),
            view_count: 50000,
            like_count: 2500,
            dislike_count: Some(12),
            comment_count: 150,
            duration_formatted: "10:30".to_string(),
            duration_seconds: 630,
            tags: vec!["rust".to_string(), "systems".to_string()],
        };

        let formatter = select_formatter::<VideoDetails>(false);
        let output = formatter.format_item(&details);
        assert!(output.contains("Deep Dive into Rust"));
        assert!(output.contains("vid_123"));
        assert!(output.contains("Views:           50000"));
        assert!(output.contains("Likes:           2500"));
        assert!(output.contains("Dislikes (RYD):  12"));
        assert!(output.contains("Tags:            rust, systems"));
    }
}
