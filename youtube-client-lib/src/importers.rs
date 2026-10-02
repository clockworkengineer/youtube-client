//! # Subscription & Playlist Interoperability (Strategy Pattern & OCP)
//!
//! Provides extensible import and export strategies for moving subscriptions between `youtube-client`,
//! standard RSS readers (via OPML), Google Takeout backups (via CSV), and alternative
//! YouTube frontends like NewPipe and FreeTube.

use std::path::Path;
use std::sync::Arc;

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

/// Strategy trait for reading and writing subscription data in various file formats (Open/Closed Principle).
pub trait SubscriptionFormat: Send + Sync {
    /// Unique machine identifier for this format (e.g. `"opml"`, `"takeout_csv"`, `"newpipe_json"`).
    fn id(&self) -> &'static str;

    /// Human-friendly display name (e.g. `"OPML XML"`, `"Google Takeout CSV"`).
    fn display_name(&self) -> &'static str;

    /// Default file extension for this format without leading dot (e.g. `"opml"`, `"csv"`, `"json"`).
    fn default_extension(&self) -> &'static str;

    /// Inspect content text and optional file path to determine if this format can parse it.
    fn can_parse(&self, content: &str, path: Option<&Path>) -> bool;

    /// Parse raw text content into a list of [`SubscriptionImport`] items.
    fn parse(&self, content: &str) -> Result<Vec<SubscriptionImport>, String>;

    /// Serialize a list of [`SubscriptionImport`] items into formatted string representation.
    fn serialize(&self, subs: &[SubscriptionImport]) -> Result<String, String>;
}

// ---------------------------------------------------------------------------
// OPML (Outline Processor Markup Language) Strategy
// ---------------------------------------------------------------------------

/// Strategy for standard OPML 1.1 / 2.0 XML files.
#[derive(Clone, Debug, Default)]
pub struct OpmlFormat;

impl SubscriptionFormat for OpmlFormat {
    fn id(&self) -> &'static str {
        "opml"
    }

    fn display_name(&self) -> &'static str {
        "OPML XML"
    }

    fn default_extension(&self) -> &'static str {
        "opml"
    }

    fn can_parse(&self, content: &str, path: Option<&Path>) -> bool {
        if let Some(p) = path {
            if let Some(ext) = p.extension() {
                if ext.eq_ignore_ascii_case("opml") || ext.eq_ignore_ascii_case("xml") {
                    return true;
                }
            }
        }
        let lower = content.to_lowercase();
        lower.contains("<opml") || (lower.contains("<outline") && lower.contains("xmlurl"))
    }

    fn parse(&self, content: &str) -> Result<Vec<SubscriptionImport>, String> {
        let mut results = Vec::new();

        for part in content.split("<outline") {
            let tag_content = part.split('>').next().unwrap_or("");
            if tag_content.is_empty() {
                continue;
            }
            let outline_tag = format!("<outline {tag_content}>");

            let channel_id = extract_attr_value(&outline_tag, "xmlUrl")
                .and_then(|url| extract_channel_id_from_url(&url))
                .or_else(|| {
                    extract_attr_value(&outline_tag, "htmlUrl")
                        .and_then(|url| extract_channel_id_from_url(&url))
                });

            let title = extract_attr_value(&outline_tag, "title")
                .or_else(|| extract_attr_value(&outline_tag, "text"))
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

    fn serialize(&self, subs: &[SubscriptionImport]) -> Result<String, String> {
        let mut out = String::with_capacity(1024 + subs.len() * 180);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<opml version=\"1.1\">\n");
        out.push_str("  <head>\n");
        out.push_str("    <title>YouTube Subscriptions</title>\n");
        out.push_str("  </head>\n");
        out.push_str("  <body>\n");
        out.push_str(
            "    <outline text=\"YouTube Subscriptions\" title=\"YouTube Subscriptions\">\n",
        );

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
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Google Takeout CSV Strategy
// ---------------------------------------------------------------------------

/// Strategy for Google Takeout `subscriptions.csv`.
#[derive(Clone, Debug, Default)]
pub struct TakeoutCsvFormat;

impl SubscriptionFormat for TakeoutCsvFormat {
    fn id(&self) -> &'static str {
        "takeout_csv"
    }

    fn display_name(&self) -> &'static str {
        "Google Takeout CSV"
    }

    fn default_extension(&self) -> &'static str {
        "csv"
    }

    fn can_parse(&self, content: &str, path: Option<&Path>) -> bool {
        if let Some(p) = path {
            if let Some(ext) = p.extension() {
                if ext.eq_ignore_ascii_case("csv") {
                    return true;
                }
            }
        }
        content
            .lines()
            .next()
            .map(|l| l.to_lowercase().contains("channel id"))
            .unwrap_or(false)
    }

    fn parse(&self, content: &str) -> Result<Vec<SubscriptionImport>, String> {
        let mut results = Vec::new();
        let mut lines = content.lines();

        // Optional header check
        if let Some(header) = lines.next() {
            if !header.to_lowercase().contains("channel id") {
                // Not standard header, process line as potential row
                if let Some(sub) = parse_csv_subscription_row(header) {
                    results.push(sub);
                }
            }
        }

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(sub) = parse_csv_subscription_row(trimmed) {
                results.push(sub);
            }
        }

        Ok(results)
    }

    fn serialize(&self, subs: &[SubscriptionImport]) -> Result<String, String> {
        let mut out = String::from("Channel Id,Channel Url,Channel Title\n");
        for sub in subs {
            let escaped_title =
                if sub.channel_title.contains(',') || sub.channel_title.contains('"') {
                    format!("\"{}\"", sub.channel_title.replace('"', "\"\""))
                } else {
                    sub.channel_title.clone()
                };
            out.push_str(&format!(
                "{},{},{}\n",
                sub.channel_id, sub.channel_url, escaped_title
            ));
        }
        Ok(out)
    }
}

fn parse_csv_subscription_row(line: &str) -> Option<SubscriptionImport> {
    let fields = parse_csv_line(line);
    if fields.len() >= 3 {
        let channel_id = fields[0].trim().to_string();
        let channel_url = fields[1].trim().to_string();
        let channel_title = fields[2].trim().to_string();

        if !channel_id.is_empty() && channel_id.to_lowercase() != "channel id" {
            return Some(SubscriptionImport {
                channel_id,
                channel_title,
                channel_url,
            });
        }
    }
    None
}

// ---------------------------------------------------------------------------
// NewPipe JSON Strategy
// ---------------------------------------------------------------------------

/// Strategy for NewPipe subscriptions backup JSON.
#[derive(Clone, Debug, Default)]
pub struct NewPipeJsonFormat;

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

impl SubscriptionFormat for NewPipeJsonFormat {
    fn id(&self) -> &'static str {
        "newpipe_json"
    }

    fn display_name(&self) -> &'static str {
        "NewPipe JSON"
    }

    fn default_extension(&self) -> &'static str {
        "json"
    }

    fn can_parse(&self, content: &str, path: Option<&Path>) -> bool {
        let trimmed = content.trim_start();
        if !trimmed.starts_with('{') {
            return false;
        }

        if let Some(p) = path {
            if let Some(ext) = p.extension() {
                if ext.eq_ignore_ascii_case("json") {
                    return true;
                }
            }
        }

        trimmed.contains("subscriptions") || trimmed.contains("app_version")
    }

    fn parse(&self, content: &str) -> Result<Vec<SubscriptionImport>, String> {
        let parsed: NewPipeExport = serde_json::from_str(content)
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

    fn serialize(&self, subs: &[SubscriptionImport]) -> Result<String, String> {
        let items = subs
            .iter()
            .map(|s| NewPipeSubscriptionItem {
                service_id: 0,
                url: s.channel_url.clone(),
                name: s.channel_title.clone(),
            })
            .collect();

        let export = NewPipeExport {
            app_version: "0.27.0".to_string(),
            app_version_int: 1000,
            subscriptions: items,
        };

        serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Universal Registry for Subscription Formats
// ---------------------------------------------------------------------------

/// Universal registry of subscription formats allowing extensible format registration
/// and automatic format detection on imports.
#[derive(Clone, Default)]
pub struct SubscriptionFormatRegistry {
    formats: Vec<Arc<dyn SubscriptionFormat>>,
}

impl SubscriptionFormatRegistry {
    /// Create an empty format registry.
    pub fn new() -> Self {
        Self {
            formats: Vec::new(),
        }
    }

    /// Create a registry pre-loaded with standard supported formats (OPML, Takeout CSV, NewPipe JSON).
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(OpmlFormat));
        registry.register(Arc::new(TakeoutCsvFormat));
        registry.register(Arc::new(NewPipeJsonFormat));
        registry
    }

    /// Register a new format strategy.
    pub fn register(&mut self, format: Arc<dyn SubscriptionFormat>) {
        self.formats.push(format);
    }

    /// Find a format by its unique ID.
    pub fn get_format(&self, id: &str) -> Option<Arc<dyn SubscriptionFormat>> {
        self.formats.iter().find(|f| f.id() == id).cloned()
    }

    /// Return all registered formats.
    pub fn formats(&self) -> &[Arc<dyn SubscriptionFormat>] {
        &self.formats
    }

    /// Automatically detect format from content / file path and parse into subscriptions.
    pub fn import_auto_detect(
        &self,
        content: &str,
        path: Option<&Path>,
    ) -> Result<Vec<SubscriptionImport>, String> {
        // 1. Try formats that identify themselves as capable of parsing the input
        for format in &self.formats {
            if format.can_parse(content, path) {
                if let Ok(subs) = format.parse(content) {
                    if !subs.is_empty() {
                        return Ok(subs);
                    }
                }
            }
        }

        // 2. Fallback: try remaining formats in sequence
        for format in &self.formats {
            if let Ok(subs) = format.parse(content) {
                if !subs.is_empty() {
                    return Ok(subs);
                }
            }
        }

        Err("Unrecognized subscription format. Supported formats: OPML XML, Google Takeout CSV, NewPipe JSON.".to_string())
    }

    /// Import subscriptions directly from a file path using auto-detection.
    pub fn import_file(&self, path: &Path) -> Result<Vec<SubscriptionImport>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {e}", path.display()))?;
        self.import_auto_detect(&content, Some(path))
    }
}

// ---------------------------------------------------------------------------
// Backward-Compatible Facade Functions
// ---------------------------------------------------------------------------

/// Automatically detect and import subscriptions from content and optional path.
pub fn import_subscriptions_auto_detect(
    content: &str,
    path: Option<&Path>,
) -> Result<Vec<SubscriptionImport>, String> {
    SubscriptionFormatRegistry::with_defaults().import_auto_detect(content, path)
}

/// Import subscriptions from standard OPML XML text.
pub fn import_subscriptions_from_opml(opml_xml: &str) -> Result<Vec<SubscriptionImport>, String> {
    OpmlFormat.parse(opml_xml)
}

/// Export subscriptions to standard OPML XML format.
pub fn export_subscriptions_to_opml(subs: &[SubscriptionImport]) -> String {
    OpmlFormat.serialize(subs).unwrap_or_default()
}

/// Import subscriptions from Google Takeout `subscriptions.csv`.
pub fn import_subscriptions_from_takeout_csv(
    csv_content: &str,
) -> Result<Vec<SubscriptionImport>, String> {
    TakeoutCsvFormat.parse(csv_content)
}

/// Export subscriptions to Google Takeout CSV format.
pub fn export_subscriptions_to_takeout_csv(subs: &[SubscriptionImport]) -> String {
    TakeoutCsvFormat.serialize(subs).unwrap_or_default()
}

/// Import subscriptions from NewPipe backup JSON format.
pub fn import_subscriptions_from_newpipe_json(
    json_content: &str,
) -> Result<Vec<SubscriptionImport>, String> {
    NewPipeJsonFormat.parse(json_content)
}

/// Export subscriptions to NewPipe backup JSON format.
pub fn export_subscriptions_to_newpipe_json(subs: &[SubscriptionImport]) -> String {
    NewPipeJsonFormat.serialize(subs).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Helper parsing functions
// ---------------------------------------------------------------------------

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn unescape_xml(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
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

    #[test]
    fn test_auto_detect_formats() {
        let registry = SubscriptionFormatRegistry::with_defaults();

        // 1. OPML detection
        let opml_content = r#"<opml version="1.1"><body><outline text="MyChan" xmlUrl="https://www.youtube.com/feeds/videos.xml?channel_id=UC_opml" /></body></opml>"#;
        let opml_subs = registry.import_auto_detect(opml_content, None).unwrap();
        assert_eq!(opml_subs.len(), 1);
        assert_eq!(opml_subs[0].channel_id, "UC_opml");

        // 2. CSV detection
        let csv_content = "Channel Id,Channel Url,Channel Title\nUC_csv,https://youtube.com/channel/UC_csv,CsvChan\n";
        let csv_subs = registry.import_auto_detect(csv_content, None).unwrap();
        assert_eq!(csv_subs.len(), 1);
        assert_eq!(csv_subs[0].channel_id, "UC_csv");

        // 3. NewPipe JSON detection
        let json_content = r#"{"app_version":"0.27.0","app_version_int":1000,"subscriptions":[{"service_id":0,"url":"https://www.youtube.com/channel/UC_json","name":"JsonChan"}]}"#;
        let json_subs = registry.import_auto_detect(json_content, None).unwrap();
        assert_eq!(json_subs.len(), 1);
        assert_eq!(json_subs[0].channel_id, "UC_json");
    }
}
