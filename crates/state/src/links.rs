use std::time::Duration;

use music::{Album, MusicApi, Playlist, Track};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderLink {
    Spotify { kind: LinkKind, id: String },
    AppleMusic { kind: LinkKind, id: String, url: String },
    YouTube { kind: LinkKind, id: String, url: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Track,
    Album,
    Playlist,
}

#[derive(Default)]
pub struct ResolvedLink {
    pub search_query: String,
    pub track: Option<Track>,
    pub album: Option<Album>,
    pub playlist: Option<Playlist>,
}

pub fn parse_link(raw: &str) -> Option<ProviderLink> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if s.starts_with("spotify:") {
        let mut parts = s.split(':');
        let _ = parts.next();
        let kind_str = parts.next()?;
        let id = parts.next()?.split('?').next()?;
        let kind = match kind_str {
            "track" => LinkKind::Track,
            "album" => LinkKind::Album,
            "playlist" => LinkKind::Playlist,
            _ => return None,
        };
        return Some(ProviderLink::Spotify {
            kind,
            id: id.to_owned(),
        });
    }

    if let Some(stripped) = s
        .strip_prefix("https://open.spotify.com/")
        .or_else(|| s.strip_prefix("http://open.spotify.com/"))
    {
        let path = stripped.split('?').next().unwrap_or(stripped);
        let mut segments: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        if segments.first().is_some_and(|seg| seg.starts_with("intl-")) {
            segments.remove(0);
        }
        let kind_str = segments.first()?;
        let id = segments.get(1)?;
        let kind = match *kind_str {
            "track" => LinkKind::Track,
            "album" => LinkKind::Album,
            "playlist" => LinkKind::Playlist,
            _ => return None,
        };
        return Some(ProviderLink::Spotify {
            kind,
            id: (*id).to_owned(),
        });
    }

    if s.starts_with("https://music.apple.com/") || s.starts_with("http://music.apple.com/") {
        let (url_base, query) = match s.split_once('?') {
            Some((b, q)) => (b, Some(q)),
            None => (s, None),
        };
        if let Some(q) = query {
            for param in q.split('&') {
                if let Some(track_id) = param.strip_prefix("i=") {
                    let clean_id = track_id.split('#').next().unwrap_or(track_id);
                    if !clean_id.is_empty() {
                        return Some(ProviderLink::AppleMusic {
                            kind: LinkKind::Track,
                            id: clean_id.to_owned(),
                            url: s.to_owned(),
                        });
                    }
                }
            }
        }
        let segments: Vec<&str> = url_base.split('/').filter(|p| !p.is_empty()).collect();
        if let Some(pos) = segments.iter().position(|&x| x == "song") {
            if let Some(id) = segments.get(pos + 2).or_else(|| segments.get(pos + 1)) {
                return Some(ProviderLink::AppleMusic {
                    kind: LinkKind::Track,
                    id: (*id).to_owned(),
                    url: s.to_owned(),
                });
            }
        }
        if let Some(pos) = segments.iter().position(|&x| x == "album") {
            if let Some(id) = segments.get(pos + 2).or_else(|| segments.get(pos + 1)) {
                return Some(ProviderLink::AppleMusic {
                    kind: LinkKind::Album,
                    id: (*id).to_owned(),
                    url: s.to_owned(),
                });
            }
        }
        if let Some(pos) = segments.iter().position(|&x| x == "playlist") {
            if let Some(id) = segments.get(pos + 2).or_else(|| segments.get(pos + 1)) {
                return Some(ProviderLink::AppleMusic {
                    kind: LinkKind::Playlist,
                    id: (*id).to_owned(),
                    url: s.to_owned(),
                });
            }
        }
        return Some(ProviderLink::AppleMusic {
            kind: LinkKind::Track,
            id: String::new(),
            url: s.to_owned(),
        });
    }

    if s.contains("youtube.com/") || s.contains("youtu.be/") {
        if s.contains("/playlist") || s.contains("list=") {
            let id = if let Some((_, q)) = s.split_once("list=") {
                q.split('&').next().unwrap_or(q).to_owned()
            } else {
                String::new()
            };
            return Some(ProviderLink::YouTube {
                kind: LinkKind::Playlist,
                id,
                url: s.to_owned(),
            });
        }
        if let Some((_, rest)) = s.split_once("youtu.be/") {
            let id = rest.split(['?', '/', '&']).next().unwrap_or(rest);
            return Some(ProviderLink::YouTube {
                kind: LinkKind::Track,
                id: id.to_owned(),
                url: s.to_owned(),
            });
        }
        if let Some((_, rest)) = s.split_once("v=") {
            let id = rest.split(['?', '&', '/']).next().unwrap_or(rest);
            return Some(ProviderLink::YouTube {
                kind: LinkKind::Track,
                id: id.to_owned(),
                url: s.to_owned(),
            });
        }
    }

    None
}

pub async fn resolve_link(link: ProviderLink, client: &dyn MusicApi) -> ResolvedLink {
    match link {
        ProviderLink::Spotify { kind, ref id } => {
            match kind {
                LinkKind::Track => {
                    if let Ok(track) = client.track(id).await {
                        let search_query = format!("{} {}", track.name, track.artists).trim().to_owned();
                        return ResolvedLink {
                            search_query,
                            track: Some(track),
                            album: None,
                            playlist: None,
                        };
                    }
                }
                LinkKind::Album => {
                    if let Ok(detail) = client.album(id).await {
                        let search_query = format!("{} {}", detail.album.name, detail.album.artists)
                            .trim()
                            .to_owned();
                        return ResolvedLink {
                            search_query,
                            track: None,
                            album: Some(detail.album),
                            playlist: None,
                        };
                    }
                }
                LinkKind::Playlist => {
                    if let Ok(detail) = client.playlist(id).await {
                        let search_query = detail.playlist.name.clone();
                        return ResolvedLink {
                            search_query,
                            track: None,
                            album: None,
                            playlist: Some(detail.playlist),
                        };
                    }
                }
            }

            let http = reqwest::Client::new();
            let query = resolve_spotify(kind, id, &http).await.unwrap_or_default();
            ResolvedLink {
                search_query: query,
                track: None,
                album: None,
                playlist: None,
            }
        }
        ProviderLink::AppleMusic { kind, ref id, ref url } => {
            if !id.is_empty() {
                match kind {
                    LinkKind::Track => {
                        if let Ok(track) = client.track(id).await {
                            let search_query = format!("{} {}", track.name, track.artists).trim().to_owned();
                            return ResolvedLink {
                                search_query,
                                track: Some(track),
                                album: None,
                                playlist: None,
                            };
                        }
                    }
                    LinkKind::Album => {
                        if let Ok(detail) = client.album(id).await {
                            let search_query = format!("{} {}", detail.album.name, detail.album.artists)
                                .trim()
                                .to_owned();
                            return ResolvedLink {
                                search_query,
                                track: None,
                                album: Some(detail.album),
                                playlist: None,
                            };
                        }
                    }
                    LinkKind::Playlist => {
                        if let Ok(detail) = client.playlist(id).await {
                            let search_query = detail.playlist.name.clone();
                            return ResolvedLink {
                                search_query,
                                track: None,
                                album: None,
                                playlist: Some(detail.playlist),
                            };
                        }
                    }
                }
            }

            let http = reqwest::Client::new();
            let query = resolve_apple_music(kind, id, url, &http).await.unwrap_or_default();
            ResolvedLink {
                search_query: query,
                track: None,
                album: None,
                playlist: None,
            }
        }
        ProviderLink::YouTube { kind, ref id, ref url } => {
            if !id.is_empty() {
                match kind {
                    LinkKind::Track => {
                        if let Ok(track) = client.track(id).await {
                            let search_query = format!("{} {}", track.name, track.artists).trim().to_owned();
                            return ResolvedLink {
                                search_query,
                                track: Some(track),
                                album: None,
                                playlist: None,
                            };
                        }
                    }
                    LinkKind::Playlist => {
                        if let Ok(detail) = client.playlist(id).await {
                            let search_query = detail.playlist.name.clone();
                            return ResolvedLink {
                                search_query,
                                track: None,
                                album: None,
                                playlist: Some(detail.playlist),
                            };
                        }
                    }
                    _ => {}
                }
            }

            let http = reqwest::Client::new();
            let query = resolve_youtube(url, &http).await.unwrap_or_default();
            ResolvedLink {
                search_query: query,
                track: None,
                album: None,
                playlist: None,
            }
        }
    }
}

async fn resolve_spotify(
    kind: LinkKind,
    id: &str,
    http: &reqwest::Client,
) -> Option<String> {
    let path = match kind {
        LinkKind::Track => "track",
        LinkKind::Album => "album",
        LinkKind::Playlist => "playlist",
    };
    let spotify_url = format!("https://open.spotify.com/{path}/{id}");

    let res = http
        .get(&spotify_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(4))
        .send()
        .await
        .ok()?;

    let html = res.text().await.ok()?;
    let artist = extract_meta(&html, "music:musician_description").or_else(|| {
        extract_meta(&html, "og:description").and_then(|desc| {
            desc.split('·').next().map(|s| s.trim().to_owned())
        })
    });
    let title = extract_meta(&html, "og:title").or_else(|| {
        extract_tag(&html, "title").map(|t| {
            t.split(" - ").next().unwrap_or(&t).trim().to_owned()
        })
    });

    match (title, artist) {
        (Some(t), Some(a)) => Some(format!("{t} {a}")),
        (Some(t), None) => Some(t),
        _ => None,
    }
}

async fn resolve_apple_music(
    _kind: LinkKind,
    id: &str,
    url: &str,
    http: &reqwest::Client,
) -> Option<String> {
    if !id.is_empty() {
        let lookup_url = format!("https://itunes.apple.com/lookup?id={id}");
        #[derive(Deserialize)]
        struct ItunesResponse {
            results: Vec<ItunesResult>,
        }
        #[derive(Deserialize)]
        struct ItunesResult {
            #[serde(rename = "trackName")]
            track_name: Option<String>,
            #[serde(rename = "artistName")]
            artist_name: Option<String>,
            #[serde(rename = "collectionName")]
            collection_name: Option<String>,
        }
        if let Ok(res) = http.get(&lookup_url).timeout(Duration::from_secs(4)).send().await
            && let Ok(data) = res.json::<ItunesResponse>().await
            && let Some(item) = data.results.into_iter().next()
        {
            if let Some(track) = item.track_name {
                let artist = item.artist_name.unwrap_or_default();
                return Some(format!("{track} {artist}").trim().to_owned());
            }
            if let Some(album) = item.collection_name {
                let artist = item.artist_name.unwrap_or_default();
                return Some(format!("{album} {artist}").trim().to_owned());
            }
        }
    }

    let oembed_url = format!("https://music.apple.com/api/oembed?url={}", urlencode(url));
    #[derive(Deserialize)]
    struct AppleOembed {
        title: Option<String>,
        author_name: Option<String>,
    }
    if let Ok(res) = http.get(&oembed_url).timeout(Duration::from_secs(4)).send().await
        && let Ok(data) = res.json::<AppleOembed>().await
    {
        let title = data.title.unwrap_or_default();
        let author = data.author_name.unwrap_or_default();
        let query = format!("{title} {author}").trim().to_owned();
        if !query.is_empty() {
            return Some(query);
        }
    }

    None
}

async fn resolve_youtube(url: &str, http: &reqwest::Client) -> Option<String> {
    let oembed_url = format!(
        "https://www.youtube.com/oembed?url={}&format=json",
        urlencode(url)
    );
    #[derive(Deserialize)]
    struct YtOembed {
        title: String,
        author_name: Option<String>,
    }
    let res = http
        .get(&oembed_url)
        .timeout(Duration::from_secs(4))
        .send()
        .await
        .ok()?;
    let data: YtOembed = res.json().await.ok()?;
    let clean = clean_youtube_title(&data.title);
    if let Some((artist, title)) = clean.split_once(" - ") {
        Some(format!("{} {}", title.trim(), artist.trim()))
    } else if let Some(author) = data.author_name {
        Some(format!("{} {}", clean, author))
    } else {
        Some(clean)
    }
}

fn clean_youtube_title(title: &str) -> String {
    let mut s = title.to_owned();
    let noises = [
        "(Official Music Video)",
        "(Official Video)",
        "(Official Audio)",
        "[Official Music Video]",
        "[Official Video]",
        "[Official Audio]",
        "(Official Visualizer)",
        "[Official Visualizer]",
        "(Visualizer)",
        "[Visualizer]",
        "(Lyrics)",
        "[Lyrics]",
        "(Lyric Video)",
        "[Lyric Video]",
        "(Audio)",
        "[Audio]",
        "(4K Remaster)",
        "(Music Video)",
        "(HD)",
        "[HD]",
    ];
    for noise in noises {
        s = s.replace(noise, "");
    }
    s.trim().to_owned()
}

fn extract_meta(html: &str, property: &str) -> Option<String> {
    let key1 = format!("property=\"{property}\" content=\"");
    let key2 = format!("name=\"{property}\" content=\"");
    let content_after = html
        .find(&key1)
        .map(|idx| idx + key1.len())
        .or_else(|| html.find(&key2).map(|idx| idx + key2.len()))?;

    let rest = &html[content_after..];
    let end = rest.find('"')?;
    let val = &rest[..end];
    if val.is_empty() {
        None
    } else {
        Some(htmlescape_decode(val))
    }
}

fn extract_tag(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = html.find(&open)? + open.len();
    let end = html[start..].find(&close)? + start;
    Some(htmlescape_decode(&html[start..end]))
}

fn htmlescape_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
