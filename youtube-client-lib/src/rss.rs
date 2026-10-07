//! # YouTube Public RSS Feed Parser (Zero-Quota Unauthenticated Fallback)
//!
//! YouTube publishes public Atom/RSS XML feeds for every channel at:
//! `https://www.youtube.com/feeds/videos.xml?channel_id=UC...`
//!
//! This module allows fetching the latest ~15 videos from any public channel without
//! requiring Google API credentials, OAuth tokens, or consuming YouTube Data API v3 quota.

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use reqwest::Client;

use crate::models::Video;

/// Parse a YouTube channel Atom/RSS XML feed string into a list of [`Video`] items using `quick-xml`.
pub fn parse_youtube_rss_xml(xml: &str) -> std::result::Result<Vec<Video>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut videos = Vec::new();
    let mut in_entry = false;
    let mut in_author = false;
    let mut tag_stack: Vec<String> = Vec::new();

    let mut current_id = String::new();
    let mut current_yt_id = String::new();
    let mut current_title = String::new();
    let mut current_media_title = String::new();
    let mut current_description = String::new();
    let mut current_published = String::new();
    let mut current_channel_title = String::new();
    let mut current_thumbnail_url = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "entry" {
                    in_entry = true;
                    current_id.clear();
                    current_yt_id.clear();
                    current_title.clear();
                    current_media_title.clear();
                    current_description.clear();
                    current_published.clear();
                    current_channel_title.clear();
                    current_thumbnail_url.clear();
                } else if in_entry && name == "author" {
                    in_author = true;
                } else if in_entry && (name == "media:thumbnail" || name.ends_with(":thumbnail") || name == "thumbnail") {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"url" {
                            if let Ok(val) = attr.decode_and_unescape_value(reader.decoder()) {
                                current_thumbnail_url = val.to_string();
                            }
                        }
                    }
                }
                tag_stack.push(name);
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if in_entry && (name == "media:thumbnail" || name.ends_with(":thumbnail") || name == "thumbnail") {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"url" {
                            if let Ok(val) = attr.decode_and_unescape_value(reader.decoder()) {
                                current_thumbnail_url = val.to_string();
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_entry {
                    if let Some(tag) = tag_stack.last() {
                        let text = e.unescape().map_err(|err| err.to_string())?.to_string();
                        match tag.as_str() {
                            "yt:videoId" | "videoId" => current_yt_id = text,
                            "id" => current_id = text,
                            "title" => current_title = text,
                            "media:title" => current_media_title = text,
                            "media:description" | "description" => current_description = text,
                            "published" => current_published = text,
                            "name" if in_author => current_channel_title = text,
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::CData(e)) => {
                if in_entry {
                    if let Some(tag) = tag_stack.last() {
                        let text = String::from_utf8_lossy(&e.into_inner()).to_string();
                        match tag.as_str() {
                            "yt:videoId" | "videoId" => current_yt_id = text,
                            "id" => current_id = text,
                            "title" => current_title = text,
                            "media:title" => current_media_title = text,
                            "media:description" | "description" => current_description = text,
                            "published" => current_published = text,
                            "name" if in_author => current_channel_title = text,
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "entry" {
                    in_entry = false;
                    let video_id = if !current_yt_id.is_empty() {
                        current_yt_id.clone()
                    } else if let Some(stripped) = current_id.strip_prefix("yt:video:") {
                        stripped.to_string()
                    } else {
                        current_id.clone()
                    };

                    if !video_id.is_empty() {
                        let title = if !current_title.is_empty() {
                            current_title.clone()
                        } else {
                            current_media_title.clone()
                        };
                        let thumbnail_url = if !current_thumbnail_url.is_empty() {
                            current_thumbnail_url.clone()
                        } else {
                            format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg")
                        };

                        videos.push(Video {
                            id: video_id,
                            title,
                            description: current_description.clone(),
                            published_at: current_published.clone(),
                            channel_title: current_channel_title.clone(),
                            thumbnail_url,
                        });
                    }
                } else if name == "author" {
                    in_author = false;
                }
                if let Some(pos) = tag_stack.iter().rposition(|t| t == &name) {
                    tag_stack.truncate(pos);
                }
            }
            Ok(Event::Eof) => {
                if !tag_stack.is_empty() {
                    return Err(format!("Unclosed XML tags at EOF: {tag_stack:?}"));
                }
                break;
            }
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
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
      <media:description><![CDATA[Second description with <special> characters]]></media:description>
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
        assert_eq!(v2.description, "Second description with <special> characters");
        assert_eq!(
            v2.thumbnail_url,
            "https://i.ytimg.com/vi/abc123xyz89/hqdefault.jpg"
        );
    }

    #[test]
    fn test_parse_youtube_rss_xml_edge_cases() {
        // Missing thumbnail should use default i.ytimg.com url
        let xml_no_thumb = r#"<feed><entry>
            <yt:videoId>test1234</yt:videoId>
            <title>Test Title</title>
        </entry></feed>"#;
        let videos = parse_youtube_rss_xml(xml_no_thumb).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].id, "test1234");
        assert_eq!(videos[0].thumbnail_url, "https://i.ytimg.com/vi/test1234/hqdefault.jpg");

        // Malformed XML returns Err
        let malformed = "<feed><entry><title>Unclosed";
        let res = parse_youtube_rss_xml(malformed);
        assert!(res.is_err());
    }
}
