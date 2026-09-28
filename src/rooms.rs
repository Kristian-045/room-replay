use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoomId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub name: String,
    pub page_url: String,
    pub stream_url: String,
}

/// Verified against https://www.fi.muni.cz/tech/video.html.cs and the linked
/// CESNET pages on 2026-09-28. S108 is intentionally excluded from the defaults.
pub fn fi_defaults() -> Vec<Room> {
    ["A217", "A218", "A318", "A319", "A320"]
        .into_iter()
        .map(|name| {
            let source = format!("munifi{}", name.to_ascii_lowercase());
            Room {
                id: RoomId(Uuid::new_v4()),
                name: format!("FI {name}"),
                page_url: format!("https://live.cesnet.cz/{source}.html"),
                stream_url: format!(
                    "https://cdn.streaming.cesnet.cz/muni/{source}.stream/playlist.m3u8"
                ),
            }
        })
        .collect()
}

fn stream_allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("cdn.streaming.cesnet.cz")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.path().ends_with(".m3u8")
}

pub async fn resolve(name: String, input: String, allow_test_sources: bool) -> Result<Room> {
    let name = name.trim().to_owned();
    if name.is_empty() || name.len() > 100 {
        bail!("Room name must contain 1 to 100 bytes");
    }
    let url = Url::parse(input.trim())?;
    let test_source = allow_test_sources
        && url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.path().ends_with(".m3u8");
    let stream_url = if test_source || stream_allowed(&url) {
        url.to_string()
    } else {
        if url.scheme() != "https"
            || url.host_str() != Some("live.cesnet.cz")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port_or_known_default() != Some(443)
        {
            bail!("Use a CESNET room page or a CESNET HLS playlist URL");
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let mut response = client.get(url.clone()).send().await?.error_for_status()?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len() + chunk.len() > 1_000_000 {
                bail!("Room page is unexpectedly large");
            }
            body.extend_from_slice(&chunk);
        }
        source_from_html(&String::from_utf8(body)?)?
    };
    Ok(Room {
        id: RoomId(Uuid::new_v4()),
        name,
        page_url: url.to_string(),
        stream_url,
    })
}

fn source_from_html(html: &str) -> Result<String> {
    let sources = regex::Regex::new(r#"(?is)<source\b[^>]*\bsrc\s*=\s*["']([^"']+)["']"#)?;
    for capture in sources.captures_iter(html) {
        if let Ok(url) = Url::parse(&capture[1])
            && stream_allowed(&url)
        {
            return Ok(url.to_string());
        }
    }
    bail!("No supported CESNET HLS source found. Try the direct playlist URL.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_page_source_without_guessing_room_path() {
        let source = "https://cdn.streaming.cesnet.cz/other/room.stream/playlist.m3u8";
        assert_eq!(
            source_from_html(&format!("<source src='{source}'>")).unwrap(),
            source
        );
        assert!(source_from_html("<source src='https://example.com/evil.m3u8'>").is_err());
    }
}
