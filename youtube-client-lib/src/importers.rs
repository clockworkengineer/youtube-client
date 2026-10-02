//! # Subscription & Playlist Interoperability (OPML, Takeout CSV, NewPipe JSON)
//!
//! Provides import and export utilities for moving subscriptions between `youtube-client`,
//! standard RSS readers (via OPML), Google Takeout backups (via CSV), and alternative
//! open-source YouTube frontends like NewPipe and FreeTube.

use serde::{Deserialize, Serialize};

use crate::models::Subscription;

/// Standardized representation of an imported or exportable YouTube subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionImport {
    /// The YouTube channel ID (usually starts with "UC").
    pub channel_id: String,
    /// The display name or title of the channel.
    pub channel_title: String,
    /// Canonical web URL for the channel.
    pub channel_url: String,
}

impl SubscriptionImport {
    /// Create a new `SubscriptionImport` record from channel ID and title.
    pub fn new(channel_id: impl Into<String>, channel_title: impl Into<String>) -> Self {
        let id = channel_id.into();
        let url = format!("https://www.youtube.com/channel/{id}");
        Self {
            channel_id: id,
            channel_title: channel_title.into(),
            channel_url: url,
        }
    }
}

impl From<&Subscription> for SubscriptionImport {
    fn from(sub: &Subscription) -> Self {
        SubscriptionImport::new(&sub.channel_id, &sub.title)
    }
}

impl From<Subscription> for SubscriptionImport {
    fn from(sub: Subscription) -> Self {
        SubscriptionImport::new(sub.channel_id, sub.title)
    }
}

/// Helper to escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Helper to unescape basic XML entities.
fn unescape_xml(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

// ---------------------------------------------------------------------------
// OPML (Outline Processor Markup Language)
// ---------------------------------------------------------------------------

/// Export subscriptions to standard OPML XML format (compatible with FreeTube, NewPipe, Feedly, and RSS readers).
pub fn export_subscriptions_to_opml(subs: &[SubscriptionImport]) -> String {
    let mut out = String::with_capacity(1024 + subs.len() * 180);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<opml version=\"1.1\">\n");
    out.push_str("  <head>\n");
    out.push_str("    <title>YouTube Subscriptions</title>\n");
    out.push_str("  </head>\n");
    out.push_str("  <body>\n");
    out.push_str("    <outline text=\"YouTube Subscriptions\" title=\"YouTube Subscriptions\">\n");

    for sub in subs {
        let title_esc = escape_xml(&sub.channel_title);
        let id_esc = escape_xml(&sub.channel_id);
        out.push_str(&format!(
            "      <outline text=\"{title_esc}\" title=\"{title_esc}\" type=\"rss\" xmlUrl=\"https://www.youtube.com/feeds/videos.xml?channel_id={id_esc}\" htmlUrl=\"https://www.youtube.com/channel/{id_esc}\" />\n"
        ));
    }

    out.push_str("    </outline>\n");
    out.push_str("  </body>\n");
    out.push_str("</opml>\n");
    out
}

/// Import subscriptions from standard OPML XML text.
pub fn import_subscriptions_from_opml(
    opml_xml: &str,
) -> std::result::Result<Vec<SubscriptionImport>, String> {
    let mut results = Vec::new();

    for line in opml_xml.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<outline") {
            continue;
        }

        // Extract xmlUrl or htmlUrl
        let channel_id = extract_attr_value(trimmed, "xmlUrl")
            .and_then(|url| extract_channel_id_from_url(&url))
            .or_else(|| {
                extract_attr_value(trimmed, "htmlUrl")
                    .and_then(|url| extract_channel_id_from_url(&url))
            });

        let title = extract_attr_value(trimmed, "title")
            .or_else(|| extract_attr_value(trimmed, "text"))
            .map(|t| unescape_xml(&t))
            .unwrap_or_else(|| "Unknown Channel".to_string());

        if let Some(id) = channel_id {
            if !id.is_empty() && id != "YouTube Subscriptions" {
                results.push(SubscriptionImport::new(id, title));
            }
        }
    }

    Ok(results)
}

fn extract_attr_value(line: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    if let Some(start) = line.find(&needle) {
        let val_start = start + needle.len();
        if let Some(end) = line[val_start..].find('"') {
            return Some(line[val_start..val_start + end].to_string());
        }
    }
    None
}

fn extract_channel_id_from_url(url: &str) -> Option<String> {
    if let Some(pos) = url.find("channel_id=") {
        let id_part = &url[pos + 11..];
        let end = id_part.find('&').unwrap_or(id_part.len());
        let id = &id_part[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    if let Some(pos) = url.find("/channel/") {
        let id_part = &url[pos + 9..];
        let end = id_part
            .find('/')
            .or_else(|| id_part.find('?'))
            .unwrap_or(id_part.len());
        let id = &id_part[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Google Takeout CSV (`subscriptions.csv`)
// ---------------------------------------------------------------------------

/// Import subscriptions from Google Takeout `subscriptions.csv`.
/// Format:
/// ```csv
/// Channel Id,Channel Url,Channel Title
/// UC_x5XG1OV2P6uZZ5FSM9Ttw,http://www.youtube.com/channel/UC_x5XG1OV2P6uZZ5FSM9Ttw,Google Developers
/// ```
pub fn import_subscriptions_from_takeout_csv(
    csv_content: &str,
) -> std::result::Result<Vec<SubscriptionImport>, String> {
    let mut results = Vec::new();
    let mut lines = csv_content.lines();

    // Check header
    if let Some(header) = lines.next() {
        if !header.to_lowercase().contains("channel id") {
            // Some Takeout files might omit header or use alternate casing
        }
    }

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse simple comma-separated fields with quote awareness
        let fields = parse_csv_line(trimmed);
        if fields.len() >= 3 {
            let channel_id = fields[0].trim().to_string();
            let channel_url = fields[1].trim().to_string();
            let channel_title = fields[2].trim().to_string();

            if !channel_id.is_empty() && channel_id.to_lowercase() != "channel id" {
                results.push(SubscriptionImport {
                    channel_id,
                    channel_title,
                    channel_url,
                });
            }
        }
    }

    Ok(results)
}

/// Export subscriptions to Google Takeout CSV format.
pub fn export_subscriptions_to_takeout_csv(subs: &[SubscriptionImport]) -> String {
    let mut out = String::from("Channel Id,Channel Url,Channel Title\n");
    for sub in subs {
        let escaped_title = if sub.channel_title.contains(',') || sub.channel_title.contains('"') {
            format!("\"{}\"", sub.channel_title.replace('"', "\"\""))
        } else {
            sub.channel_title.clone()
        };
        out.push_str(&format!(
            "{},{},{}\n",
            sub.channel_id, sub.channel_url, escaped_title
        ));
    }
    out
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                chars.next();
                current.push('"');
            } else {
                in_quotes = !in_quotes;
            }
        } else if c == ',' && !in_quotes {
            fields.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    fields.push(current.trim().to_string());
    fields
}

// ---------------------------------------------------------------------------
// NewPipe JSON Format
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct NewPipeExport {
    app_version: String,
    app_version_int: u32,
    subscriptions: Vec<NewPipeSubscriptionItem>,
}

#[derive(Serialize, Deserialize)]
struct NewPipeSubscriptionItem {
    service_id: u32,
    url: String,
    name: String,
}

/// Import subscriptions from NewPipe backup JSON format.
pub fn import_subscriptions_from_newpipe_json(
    json_content: &str,
) -> std::result::Result<Vec<SubscriptionImport>, String> {
    let parsed: NewPipeExport = serde_json::from_str(json_content)
        .map_err(|e| format!("Failed to parse NewPipe subscriptions JSON: {e}"))?;

    let mut results = Vec::new();
    for item in parsed.subscriptions {
        let channel_id = extract_channel_id_from_url(&item.url)
            .unwrap_or_else(|| item.url.replace("https://www.youtube.com/channel/", ""));
        results.push(SubscriptionImport {
            channel_id,
            channel_title: item.name,
            channel_url: item.url,
        });
    }

    Ok(results)
}

/// Export subscriptions to NewPipe backup JSON format.
pub fn export_subscriptions_to_newpipe_json(subs: &[SubscriptionImport]) -> String {
    let items = subs
        .iter()
        .map(|s| NewPipeSubscriptionItem {
            service_id: 0, // YouTube service ID in NewPipe
            url: s.channel_url.clone(),
            name: s.channel_title.clone(),
        })
        .collect();

    let export = NewPipeExport {
        app_version: "0.27.0".to_string(),
        app_version_int: 1000,
        subscriptions: items,
    };

    serde_json::to_string_pretty(&export).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opml_roundtrip() {
        let subs = vec![
            SubscriptionImport::new("UC_x5XG1OV2P6uZZ5FSM9Ttw", "Google Developers & Code"),
            SubscriptionImport::new("UCBJycsmduvYEL83R_U4JriQ", "MKBHD"),
        ];

        let opml = export_subscriptions_to_opml(&subs);
        assert!(opml.contains("Google Developers &amp; Code"));
        assert!(opml.contains("xmlUrl=\"https://www.youtube.com/feeds/videos.xml?channel_id=UC_x5XG1OV2P6uZZ5FSM9Ttw\""));

        let imported = import_subscriptions_from_opml(&opml).unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].channel_id, "UC_x5XG1OV2P6uZZ5FSM9Ttw");
        assert_eq!(imported[0].channel_title, "Google Developers & Code");
        assert_eq!(imported[1].channel_id, "UCBJycsmduvYEL83R_U4JriQ");
        assert_eq!(imported[1].channel_title, "MKBHD");
    }

    #[test]
    fn test_takeout_csv_roundtrip() {
        let subs = vec![
            SubscriptionImport::new("UC12345", "Channel with, comma"),
            SubscriptionImport::new("UC67890", "Channel with \"quotes\""),
        ];

        let csv = export_subscriptions_to_takeout_csv(&subs);
        let imported = import_subscriptions_from_takeout_csv(&csv).unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].channel_id, "UC12345");
        assert_eq!(imported[0].channel_title, "Channel with, comma");
        assert_eq!(imported[1].channel_id, "UC67890");
        assert_eq!(imported[1].channel_title, "Channel with \"quotes\"");
    }

    #[test]
    fn test_newpipe_json_roundtrip() {
        let subs = vec![
            SubscriptionImport::new("UC_channel1", "First Channel"),
            SubscriptionImport::new("UC_channel2", "Second Channel"),
        ];

        let json = export_subscriptions_to_newpipe_json(&subs);
        let imported = import_subscriptions_from_newpipe_json(&json).unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].channel_id, "UC_channel1");
        assert_eq!(imported[0].channel_title, "First Channel");
        assert_eq!(imported[1].channel_id, "UC_channel2");
        assert_eq!(imported[1].channel_title, "Second Channel");
    }
}
