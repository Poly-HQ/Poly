use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use anyhow::{Context as _, Result};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Track;

const ALBUMS: &str = "https://api.deezer.com/search/album";
const TRACKS: &str = "https://api.deezer.com/search/track";
const RESULTS: &str = "10";

/// Finds cover art by name in Deezer's public catalogue, which needs no account and serves every
/// image from a public https url. It is for a track whose own art cannot be handed to someone
/// else, so a miss is `None` rather than an error.
#[derive(Clone, Default)]
pub struct ArtworkSearch {
    http: reqwest::Client,
}

/// What a lookup searches for. A track with an album is looked up by the album, so every track
/// of one album shares a query and a result. A track without one is looked up by its title.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArtworkQuery {
    artist: String,
    album: String,
    title: String,
}

#[derive(Deserialize)]
struct Page<T> {
    #[serde(default = "Vec::new")]
    data: Vec<T>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct FoundAlbum {
    title: String,
    artist: Named,
    cover_xl: Option<String>,
}

#[derive(Deserialize)]
struct FoundTrack {
    title: String,
    artist: Named,
    album: Covered,
}

#[derive(Deserialize)]
struct Named {
    name: String,
}

#[derive(Deserialize)]
struct Covered {
    cover_xl: Option<String>,
}

impl ArtworkQuery {
    /// The query for `track`, or `None` when it has no artist or nothing else to search by.
    pub fn for_track(track: &Track) -> Option<Self> {
        let artist = track
            .artist_refs
            .first()
            .map(|artist| artist.name.as_str())
            .or_else(|| track.artists.split(", ").next())
            .map(searchable)
            .unwrap_or_default();
        let album = searchable(&track.album);
        let title = match album.is_empty() {
            true => searchable(&track.name),
            false => String::new(),
        };
        if artist.is_empty() || album.is_empty() && title.is_empty() {
            return None;
        }
        Some(Self {
            artist,
            album,
            title,
        })
    }
}

impl ArtworkSearch {
    pub fn new() -> Self {
        Self::default()
    }

    /// The url of a cover for `query`, or `None` when no result names the same artist and a
    /// close enough album or title to trust. A wrong cover is worse than none.
    pub async fn find(&self, query: &ArtworkQuery) -> Result<Option<String>> {
        match query.album.is_empty() {
            false => {
                let found: Vec<FoundAlbum> = self.search(ALBUMS, &query.album, query).await?;
                let candidates = found
                    .into_iter()
                    .filter(|album| same_artist(&album.artist.name, &query.artist))
                    .map(|album| (album.title, album.cover_xl));
                Ok(closest(candidates, &query.album))
            }
            true => {
                let found: Vec<FoundTrack> = self.search(TRACKS, &query.title, query).await?;
                let candidates = found
                    .into_iter()
                    .filter(|track| same_artist(&track.artist.name, &query.artist))
                    .map(|track| (track.title, track.album.cover_xl));
                Ok(closest(candidates, &query.title))
            }
        }
    }

    /// One page of results for the artist and `name`. Deezer reports a refused search, such as
    /// one over its rate limit, in the body of a successful response.
    async fn search<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        name: &str,
        query: &ArtworkQuery,
    ) -> Result<Vec<T>> {
        let terms = format!("{} {name}", query.artist);
        let response = self
            .http
            .get(endpoint)
            .query(&[("q", terms.as_str()), ("limit", RESULTS)])
            .send()
            .await
            .context("cannot reach deezer")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("deezer answered with status {status}");
        }
        let page: Page<T> = response
            .json()
            .await
            .context("cannot read the deezer search")?;
        if let Some(error) = page.error {
            anyhow::bail!("deezer refused the search: {error}");
        }
        Ok(page.data)
    }
}

/// The cover of the candidate whose name matches `wanted` exactly, or failing that the first one
/// that only adds words to it, as an edition or a feature credit does.
fn closest(
    candidates: impl Iterator<Item = (String, Option<String>)>,
    wanted: &str,
) -> Option<String> {
    let wanted = compared(wanted);
    let mut near = None;
    for (name, cover) in candidates {
        let Some(cover) = cover.filter(|cover| cover.starts_with("https://")) else {
            continue;
        };
        let name = compared(&searchable(&name));
        if name == wanted {
            return Some(cover);
        }
        if near.is_none() && extends(&name, &wanted) {
            near = Some(cover);
        }
    }
    near
}

fn same_artist(found: &str, wanted: &str) -> bool {
    let found = compared(found);
    let wanted = compared(wanted);
    found == wanted || extends(&found, &wanted)
}

/// Whether one of two compared names is the other with whole words added after it.
fn extends(one: &str, other: &str) -> bool {
    let starts = |long: &str, short: &str| {
        !short.is_empty()
            && long
                .strip_prefix(short)
                .is_some_and(|rest| rest.starts_with(' '))
    };
    starts(one, other) || starts(other, one)
}

/// `text` without anything in brackets, since Deezer wants every word of a search to match and
/// "(Official Video)" or "[Remastered]" would rule out the release itself.
fn searchable(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut depth = 0usize;
    for character in text.chars() {
        match character {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => kept.push(character),
            _ => {}
        }
    }
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `text` lowercased, with every run of punctuation and spaces turned into one space.
fn compared(text: &str) -> String {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

const APPLE_CATALOG_BASE: &str = "https://amp-api.music.apple.com/v1/catalog";
const APPLE_ORIGIN: &str = "https://music.apple.com";
const APPLE_REFERER: &str = "https://music.apple.com/";
const MAX_MOTION_BYTES: u64 = 3_500_000;

static MOTION_HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn detect_storefront(text: &str) -> &'static str {
    for ch in text.chars() {
        match ch as u32 {
            0x0E00..=0x0E7F => return "th",
            0x3040..=0x30FF | 0x31F0..=0x31FF => return "jp",
            0xAC00..=0xD7AF | 0x1100..=0x11FF => return "kr",
            _ => {}
        }
    }
    "us"
}

#[derive(Deserialize)]
struct AppleDocument<T> {
    #[serde(default = "Vec::new")]
    data: Vec<T>,
}

#[derive(Deserialize)]
struct AppleSearchResponse {
    results: Option<AppleSearchResults>,
}

#[derive(Deserialize)]
struct AppleSearchResults {
    albums: Option<AppleDocument<AppleAlbumItem>>,
    songs: Option<AppleDocument<AppleSongItem>>,
}

#[derive(Deserialize)]
struct AppleAlbumItem {
    attributes: Option<AppleAlbumAttributes>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppleAlbumAttributes {
    name: String,
    artist_name: String,
    editorial_video: Option<AppleEditorialVideo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppleEditorialVideo {
    motion_square_video1x1: Option<AppleMotionAsset>,
    motion_detail_square: Option<AppleMotionAsset>,
}

#[derive(Deserialize)]
struct AppleMotionAsset {
    video: Option<String>,
}

#[derive(Deserialize)]
struct AppleSongItem {
    attributes: Option<AppleSongAttributes>,
    relationships: Option<AppleSongRelationships>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppleSongAttributes {
    name: String,
    artist_name: String,
}

#[derive(Deserialize)]
struct AppleSongRelationships {
    albums: Option<AppleDocument<AppleIdItem>>,
}

#[derive(Deserialize)]
struct AppleIdItem {
    id: String,
}

pub async fn motion_artwork(album_id: Option<&str>, track: &Track) -> Option<String> {
    println!(
        "[motion-artwork] Starting lookup: name='{}', artists='{}', album='{}', album_id={:?}",
        track.name, track.artists, track.album, album_id
    );
    let http = MOTION_HTTP.get_or_init(reqwest::Client::new);
    let token = match crate::apple::auth::bearer(http).await {
        Ok(t) => {
            println!("[motion-artwork] Apple Music bearer token acquired ({} chars)", t.len());
            t
        }
        Err(err) => {
            println!("[motion-artwork] Failed to acquire Apple Music bearer token: {err:#}");
            return None;
        }
    };
    let playlist = match resolve_motion_playlist(http, &token, album_id, track).await {
        Some(p) => {
            println!("[motion-artwork] Resolved motion playlist: {}", p);
            p
        }
        None => {
            println!(
                "[motion-artwork] No motion video found for '{}' by '{}'",
                track.name, track.artists
            );
            return None;
        }
    };
    let mp4_result = download_motion_mp4(http, &playlist).await;
    match &mp4_result {
        Some(path) => println!("[motion-artwork] Motion MP4 ready at: {}", path),
        None => println!("[motion-artwork] Failed to download motion MP4 from {}", playlist),
    }
    mp4_result
}

async fn resolve_motion_playlist(
    http: &reqwest::Client,
    token: &str,
    album_id: Option<&str>,
    track: &Track,
) -> Option<String> {
    let artist = track
        .artist_refs
        .first()
        .map(|artist| artist.name.as_str())
        .or_else(|| track.artists.split(", ").next())
        .map(searchable)
        .unwrap_or_default();
    let album = searchable(&track.album);
    let title = searchable(&track.name);

    let mut storefronts = Vec::new();
    let detected = detect_storefront(&format!("{} {} {}", track.name, track.artists, track.album));
    storefronts.push(detected);
    if detected != "us" {
        storefronts.push("us");
    } else {
        storefronts.push("th");
    }

    for storefront in storefronts {
        println!("[motion-artwork] Trying storefront '{}'", storefront);
        if let Some(id) = album_id.filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
            && let Some(video) = fetch_apple_album_motion(http, token, storefront, id).await
        {
            return Some(video);
        }

        if !album.is_empty() {
            let term = if artist.is_empty() {
                album.clone()
            } else {
                format!("{artist} {album}")
            };
            if let Some(video) = search_apple_motion(http, token, storefront, &term, &artist, &album, &title).await {
                return Some(video);
            }
        }

        if !title.is_empty() && title != album {
            let term = if artist.is_empty() {
                title.clone()
            } else {
                format!("{artist} {title}")
            };
            if let Some(video) = search_apple_motion(http, token, storefront, &term, &artist, &album, &title).await {
                return Some(video);
            }
        }
    }

    None
}

async fn fetch_apple_album_motion(
    http: &reqwest::Client,
    token: &str,
    storefront: &str,
    album_id: &str,
) -> Option<String> {
    let url = format!("{APPLE_CATALOG_BASE}/{storefront}/albums/{album_id}");
    println!("[motion-artwork] Requesting album motion: {}", url);
    let resp = http
        .get(url)
        .bearer_auth(token)
        .header(reqwest::header::ORIGIN, APPLE_ORIGIN)
        .header(reqwest::header::REFERER, APPLE_REFERER)
        .header(reqwest::header::USER_AGENT, crate::apple::auth::AGENT)
        .query(&[("extend", "editorialVideo")])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        println!("[motion-artwork] Album query status: {}", resp.status());
        return None;
    }
    let doc: AppleDocument<AppleAlbumItem> = resp.json().await.ok()?;
    let video = doc.data
        .into_iter()
        .find_map(|item| extract_editorial_video(item.attributes?.editorial_video));
    if let Some(v) = &video {
        println!("[motion-artwork] Album id {} has editorial video: {}", album_id, v);
    }
    video
}

async fn search_apple_motion(
    http: &reqwest::Client,
    token: &str,
    storefront: &str,
    term: &str,
    artist: &str,
    album: &str,
    title: &str,
) -> Option<String> {
    let url = format!("{APPLE_CATALOG_BASE}/{storefront}/search");
    println!("[motion-artwork] Searching catalog ({storefront}) term: '{}'", term);
    let resp = http
        .get(url)
        .bearer_auth(token)
        .header(reqwest::header::ORIGIN, APPLE_ORIGIN)
        .header(reqwest::header::REFERER, APPLE_REFERER)
        .header(reqwest::header::USER_AGENT, crate::apple::auth::AGENT)
        .query(&[
            ("term", term),
            ("types", "albums,songs"),
            ("limit", "10"),
            ("extend", "editorialVideo"),
        ])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        println!("[motion-artwork] Search query status: {}", resp.status());
        return None;
    }
    let resp: AppleSearchResponse = resp.json().await.ok()?;
    let results = resp.results?;

    if let Some(albums) = results.albums {
        println!("[motion-artwork] Found {} candidate album(s)", albums.data.len());
        let matched = albums
            .data
            .into_iter()
            .filter_map(|item| item.attributes)
            .filter(|attr| artist.is_empty() || same_artist(&attr.artist_name, artist))
            .filter_map(|attr| {
                let video = extract_editorial_video(attr.editorial_video)?;
                println!("[motion-artwork] Album candidate '{}' has motion video", attr.name);
                Some((attr.name, Some(video)))
            })
            .collect::<Vec<_>>();
        if !album.is_empty()
            && let Some(video) = closest(matched.iter().cloned(), album)
        {
            return Some(video);
        }
        if !title.is_empty()
            && let Some(video) = closest(matched.iter().cloned(), title)
        {
            return Some(video);
        }
        if matched.len() == 1 {
            return matched.into_iter().next().and_then(|(_, v)| v);
        }
    }

    if let Some(songs) = results.songs {
        println!("[motion-artwork] Found {} candidate song(s)", songs.data.len());
        for song in songs.data {
            let Some(attr) = song.attributes else {
                continue;
            };
            if !artist.is_empty() && !same_artist(&attr.artist_name, artist) {
                continue;
            }
            let song_name = compared(&searchable(&attr.name));
            let wanted = compared(if title.is_empty() { album } else { title });
            if !wanted.is_empty() && song_name != wanted && !extends(&song_name, &wanted) {
                continue;
            }
            if let Some(album_id) = song
                .relationships
                .and_then(|rel| rel.albums)
                .and_then(|albums| albums.data.into_iter().next())
                .map(|item| item.id)
                && let Some(video) = fetch_apple_album_motion(http, token, storefront, &album_id).await
            {
                return Some(video);
            }
        }
    }

    None
}

fn extract_editorial_video(editorial: Option<AppleEditorialVideo>) -> Option<String> {
    let editorial = editorial?;
    editorial
        .motion_square_video1x1
        .and_then(|asset| asset.video)
        .or_else(|| editorial.motion_detail_square.and_then(|asset| asset.video))
        .filter(|url| url.starts_with("https://"))
}

async fn download_motion_mp4(http: &reqwest::Client, master_url: &str) -> Option<String> {
    let mut hasher = DefaultHasher::new();
    master_url.hash(&mut hasher);
    let key = hasher.finish();
    let dir = std::env::temp_dir().join("poly-motion");
    std::fs::create_dir_all(&dir).ok()?;
    let target = dir.join(format!("{key:016x}.mp4"));
    if std::fs::metadata(&target).is_ok_and(|meta| meta.len() > 1024) {
        println!("[motion-artwork] Using disk-cached MP4: {}", target.display());
        return Some(format!("file://{}", target.display()));
    }

    println!("[motion-artwork] Fetching master playlist: {}", master_url);
    let master_text = http
        .get(master_url)
        .header(reqwest::header::USER_AGENT, crate::apple::auth::AGENT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .await
        .ok()?;
    let variant_uri = pick_square_avc1_variant(&master_text)?;
    let variant_url = resolve_relative_url(master_url, &variant_uri)?;
    println!("[motion-artwork] Picked variant stream: {}", variant_url);

    let variant_text = http
        .get(&variant_url)
        .header(reqwest::header::USER_AGENT, crate::apple::auth::AGENT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .await
        .ok()?;
    let (mp4_uri, end_byte) = parse_variant_mp4_range(&variant_text)?;
    let mp4_url = resolve_relative_url(&variant_url, &mp4_uri)?;
    println!("[motion-artwork] Downloading MP4 fragment: {}, max_end: {}", mp4_url, end_byte);

    let mut request = http
        .get(&mp4_url)
        .header(reqwest::header::USER_AGENT, crate::apple::auth::AGENT);
    if end_byte > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes=0-{end_byte}"));
    }
    let bytes = request
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .bytes()
        .await
        .ok()?;
    if bytes.len() <= 1024 {
        println!("[motion-artwork] Downloaded MP4 is too small ({} bytes)", bytes.len());
        return None;
    }

    let tmp = dir.join(format!("{key:016x}.tmp"));
    std::fs::write(&tmp, &bytes).ok()?;
    let _ = std::fs::rename(&tmp, &target);
    if !target.exists() {
        std::fs::write(&target, &bytes).ok()?;
    }
    println!("[motion-artwork] Successfully saved {} bytes to {}", bytes.len(), target.display());
    Some(format!("file://{}", target.display()))
}

fn pick_square_avc1_variant(master: &str) -> Option<String> {
    let mut best: Option<(i32, u64, String)> = None;
    let mut lines = master.lines().map(str::trim);
    while let Some(line) = lines.next() {
        let Some(info) = line.strip_prefix("#EXT-X-STREAM-INF:") else {
            continue;
        };
        let uri = lines
            .by_ref()
            .find(|next| !next.is_empty() && !next.starts_with('#'))?;
        let info_lower = info.to_ascii_lowercase();
        if !info_lower.contains("avc1") && !info_lower.contains("hvc1") {
            continue;
        }
        let Some((width, height)) = parse_m3u8_resolution(info) else {
            continue;
        };
        let aspect_diff = (width as i32 - height as i32).abs();
        if aspect_diff > 8 && (width as f32 / height as f32 - 1.0).abs() > 0.05 {
            continue;
        }
        if width < 240 {
            continue;
        }
        let bandwidth = parse_m3u8_u64(info, "BANDWIDTH=").unwrap_or(0);
        let distance = (width as i32 - 408).abs();
        let replace = match &best {
            None => true,
            Some((best_dist, best_bw, _)) => {
                distance < *best_dist || (distance == *best_dist && bandwidth > *best_bw)
            }
        };
        if replace {
            best = Some((distance, bandwidth, uri.to_owned()));
        }
    }
    best.map(|(_, _, uri)| uri)
}

fn parse_m3u8_resolution(info: &str) -> Option<(u32, u32)> {
    let rest = info.split("RESOLUTION=").nth(1)?;
    let token = rest.split(',').next()?.trim();
    let (w, h) = token.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn parse_m3u8_u64(info: &str, key: &str) -> Option<u64> {
    let rest = info.split(key).nth(1)?;
    let token = rest
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .trim();
    token.parse().ok()
}

fn parse_variant_mp4_range(variant: &str) -> Option<(String, u64)> {
    let mut uri = None;
    let mut max_end = 0u64;
    let mut segments = 0usize;

    for line in variant.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("#EXT-X-MAP:") {
            if let Some(u) = extract_quoted_attr(rest, "URI=\"") {
                uri = Some(u);
            }
            if let Some(range) = extract_quoted_attr(rest, "BYTERANGE=\"")
                && let Some(end) = parse_byterange_end(&range)
            {
                max_end = max_end.max(end);
            }
        } else if let Some(range) = line.strip_prefix("#EXT-X-BYTERANGE:") {
            if let Some(end) = parse_byterange_end(range) {
                if segments == 0 || end <= MAX_MOTION_BYTES {
                    max_end = max_end.max(end);
                    segments += 1;
                }
            }
        } else if !line.is_empty() && !line.starts_with('#') && uri.is_none() {
            uri = Some(line.to_owned());
        }
    }

    Some((uri?, max_end))
}

fn extract_quoted_attr(line: &str, prefix: &str) -> Option<String> {
    let rest = line.split(prefix).nth(1)?;
    Some(rest.split('"').next()?.to_owned())
}

fn parse_byterange_end(spec: &str) -> Option<u64> {
    let (len_str, offset_str) = spec.trim().split_once('@')?;
    let len: u64 = len_str.parse().ok()?;
    let offset: u64 = offset_str.parse().ok()?;
    Some(offset.saturating_add(len).saturating_sub(1))
}

fn resolve_relative_url(base: &str, target: &str) -> Option<String> {
    if target.starts_with("https://") || target.starts_with("http://") {
        return Some(target.to_owned());
    }
    let slash = base.rfind('/')?;
    Some(format!("{}/{}", &base[..slash], target))
}
