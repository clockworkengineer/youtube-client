//! # YouTube Public RSS Feed Parser (Zero-Quota Unauthenticated Fallback)
//!
//! YouTube publishes public Atom/RSS XML feeds for every channel at:
//! `https://www.youtube.com/feeds/videos.xml?channel_id=UC...`
//!
//! This module allows fetching the latest ~15 videos from any public channel without
//! requiring Google API credentials, OAuth tokens, or consuming YouTube Data API v3 quota.

use reqwest::Client;

use crate::models::Video;

/// Parse a YouTube channel Atom/RSS XML feed string into a list of [`Video`] items.
pub fn parse_youtube_rss_xml(xml: &str) -> std::result::Result<Vec<Video>, String> {
    let mut videos = Vec::new();

    // Split into <entry> blocks
    let mut entries = xml.split("<entry>");
    // Skip before the first <entry> (the feed header)
    entries.next();

    for entry_str in entries {
        let entry = match entry_str.find("</entry>") {
            Some(end) => &entry_str[..end],
            None => entry_str,
        };

        let video_id = extract_tag_value(entry, "yt:videoId").or_else(|| {
            extract_tag_value(entry, "id")
                .and_then(|id| id.strip_prefix("yt:video:").map(|s| s.to_string()))
        });

        let title = extract_tag_value(entry, "title")
            .or_else(|| extract_tag_value(entry, "media:title"))
            .map(|t| unescape_xml(&t))
            .unwrap_or_default();

        let description = extract_tag_value(entry, "media:description")
            .map(|d| unescape_xml(&d))
            .unwrap_or_default();

        let published_at = extract_tag_value(entry, "published").unwrap_or_default();

        let channel_title = extract_tag_value(entry, "name")
            .map(|n| unescape_xml(&n))
            .unwrap_or_default();

        let thumbnail_url =
            extract_attr_from_tag(entry, "media:thumbnail", "url").unwrap_or_else(|| {
                if let Some(ref id) = video_id {
                    format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg")
                } else {
                    String::new()
                }
            });

        if let Some(id) = video_id {
            if !id.is_empty() {
                videos.push(Video {
                    id,
                    title,
                    description,
                    published_at,
                    channel_title,
                    thumbnail_url,
                });
            }
        }
    }

    Ok(videos)
}

/// Fetch the latest videos from a YouTube channel via its public Atom/RSS feed with zero API quota.
pub async fn fetch_channel_videos_via_rss(
    http_client: &Client,
    channel_id: &str,
) -> std::result::Result<Vec<Video>, String> {
    let url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}");
    let response = http_client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send()
        .await
        .map_err(|e| format!("Failed to request public RSS feed for channel {channel_id}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Public RSS feed returned status {} for channel {channel_id}",
            response.status()
        ));
    }

    let xml = response
        .text()
        .await
        .map_err(|e| format!("Failed to read RSS feed body for channel {channel_id}: {e}"))?;

    parse_youtube_rss_xml(&xml)
}

fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    if let Some(start) = xml.find(&open_tag) {
        let content_start = start + open_tag.len();
        if let Some(end) = xml[content_start..].find(&close_tag) {
            return Some(xml[content_start..content_start + end].trim().to_string());
        }
    }
    None
}

fn extract_attr_from_tag(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let open_tag = format!("<{tag}");
    if let Some(start) = xml.find(&open_tag) {
        let after_tag = &xml[start + open_tag.len()..];
        if let Some(tag_end) = after_tag.find('>') {
            let tag_content = &after_tag[..tag_end];
            let attr_needle = format!("{attr}=\"");
            if let Some(attr_start) = tag_content.find(&attr_needle) {
                let val_start = attr_start + attr_needle.len();
                if let Some(val_end) = tag_content[val_start..].find('"') {
                    return Some(tag_content[val_start..val_start + val_end].to_string());
                }
            }
        }
    }
    None
}

fn unescape_xml(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns:yt="http://www.youtube.com/xml/schemas/2015" xmlns:media="http://search.yahoo.com/mrss/" xmlns="http://www.w3.org/2005/Atom">
  <link rel="self" href="http://www.youtube.com/feeds/videos.xml?channel_id=UC_x5XG1OV2P6uZZ5FSM9Ttw"/>
  <title>Google Developers</title>
  <entry>
    <id>yt:video:dQw4w9WgXcQ</id>
    <yt:videoId>dQw4w9WgXcQ</yt:videoId>
    <yt:channelId>UC_x5XG1OV2P6uZZ5FSM9Ttw</yt:channelId>
    <title>Never Gonna Give You Up &amp; More</title>
    <published>2024-03-01T12:00:00+00:00</published>
    <author>
      <name>Rick Astley</name>
    </author>
    <media:group>
      <media:title>Never Gonna Give You Up &amp; More</media:title>
      <media:thumbnail url="https://i1.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg" width="480" height="360"/>
      <media:description>The legendary music video.</media:description>
    </media:group>
  </entry>
  <entry>
    <id>yt:video:abc123xyz89</id>
    <yt:videoId>abc123xyz89</yt:videoId>
    <yt:channelId>UC_x5XG1OV2P6uZZ5FSM9Ttw</yt:channelId>
    <title>Second Video</title>
    <published>2024-03-02T12:00:00+00:00</published>
    <author>
      <name>Rick Astley</name>
    </author>
    <media:group>
      <media:description>Second description</media:description>
    </media:group>
  </entry>
</feed>"#;

    #[test]
    fn test_parse_youtube_rss_xml() {
        let videos = parse_youtube_rss_xml(SAMPLE_RSS).unwrap();
        assert_eq!(videos.len(), 2);

        let v1 = &videos[0];
        assert_eq!(v1.id, "dQw4w9WgXcQ");
        assert_eq!(v1.title, "Never Gonna Give You Up & More");
        assert_eq!(v1.channel_title, "Rick Astley");
        assert_eq!(v1.published_at, "2024-03-01T12:00:00+00:00");
        assert_eq!(v1.description, "The legendary music video.");
        assert_eq!(
            v1.thumbnail_url,
            "https://i1.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg"
        );

        let v2 = &videos[1];
        assert_eq!(v2.id, "abc123xyz89");
        assert_eq!(v2.title, "Second Video");
        assert_eq!(
            v2.thumbnail_url,
            "https://i.ytimg.com/vi/abc123xyz89/hqdefault.jpg"
        );
    }
}
