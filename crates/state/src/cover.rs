use std::collections::HashMap;

use gpui::{Context, Entity, Task};

use crate::{Io, Playback, Session, SessionEvent, join};

pub struct Cover {
    session: Entity<Session>,
    playback: Entity<Playback>,
    key: Option<String>,
    album: Option<String>,
    large: Option<String>,
    motion: Option<String>,
    cache: HashMap<String, String>,
    motion_cache: HashMap<String, Option<String>>,
    io: Io,
    task: Option<Task<()>>,
}

impl Cover {
    pub fn new(
        session: Entity<Session>,
        playback: Entity<Playback>,
        io: Io,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&playback, |this, _, cx| this.follow(cx))
            .detach();
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedOut => this.forget(cx),
            SessionEvent::SignedIn | SessionEvent::Reconnected | SessionEvent::LocalChanged => {}
        })
        .detach();

        Self {
            session,
            playback,
            key: None,
            album: None,
            large: None,
            motion: None,
            cache: HashMap::new(),
            motion_cache: HashMap::new(),
            io,
            task: None,
        }
    }

    pub fn large(&self) -> Option<&str> {
        self.large.as_deref()
    }

    /// Returns the large artwork only when it belongs to `album`.
    pub(crate) fn large_for(&self, album: &str) -> Option<&str> {
        self.large
            .as_deref()
            .filter(|_| self.album.as_deref() == Some(album))
    }

    fn forget(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        self.key = None;
        self.album = None;
        self.large = None;
        self.motion = None;
        self.cache.clear();
        cx.notify();
    }

    fn follow(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.playback.read(cx).track().cloned() else {
            if self.key.is_some() {
                self.task = None;
                self.key = None;
                self.album = None;
                self.large = None;
                self.motion = None;
                cx.notify();
            }
            return;
        };
        let album = track.album_id.clone();
        let key = album
            .clone()
            .or_else(|| track.id.clone())
            .unwrap_or_else(|| format!("{}|{}|{}", track.artists, track.album, track.name));
        if self.key.as_deref() == Some(key.as_str()) {
            return;
        }
        self.task = None;
        self.key = Some(key.clone());
        self.album = album.clone();
        self.large = album.as_ref().and_then(|id| self.cache.get(id)).cloned();
        let cached_motion = self.motion_cache.get(&key).cloned();
        self.motion = cached_motion.clone().flatten();
        cx.notify();

        let need_large = album.is_some() && self.large.is_none();
        let need_motion = false;
        if !need_large && !need_motion {
            return;
        }
        self.load(key, album, track, need_large, need_motion, cx);
    }

    fn load(
        &mut self,
        key: String,
        album: Option<String>,
        track: music::Track,
        need_large: bool,
        need_motion: bool,
        cx: &mut Context<Self>,
    ) {
        let session = self.session.read(cx);
        let client = album.as_deref().and_then(|id| match music::is_local_id(id) {
            true => session.local_client(),
            false => session.client(),
        });
        let io = self.io.clone();

        self.task = Some(cx.spawn(async move |this, cx| {
            if need_large
                && let (Some(client), Some(id)) = (client, album.clone())
            {
                let io_album = io.clone();
                let wanted = id.clone();
                let found = join(io_album.spawn(async move { client.album(&wanted).await })).await;
                let _ = this.update(cx, |this, cx| match found {
                    Ok(detail) => {
                        if let Some(large) = detail.album.cover_large {
                            this.cache.insert(id.clone(), large.clone());
                            if this.album.as_deref() == Some(id.as_str()) {
                                this.large = Some(large);
                                cx.notify();
                            }
                        }
                    }
                    Err(error) => {
                        log::warn!("cover: cannot load {id}: {error:#}");
                        crate::noted(&error, cx);
                    }
                });
            }

            if need_motion {
                let album_ref = album.clone();
                let found_motion = io
                    .spawn(async move {
                        music::artwork::motion_artwork(album_ref.as_deref(), &track).await
                    })
                    .await
                    .ok()
                    .flatten();

                let _ = this.update(cx, |this, cx| {
                    this.task = None;
                    this.motion_cache.insert(key.clone(), found_motion.clone());
                    if this.key.as_deref() == Some(key.as_str()) && found_motion.is_some() {
                        println!(
                            "[motion-artwork] Applying motion artwork to Cover state: {:?}",
                            found_motion
                        );
                        this.motion = found_motion;
                        cx.notify();
                    } else if found_motion.is_none() {
                        println!("[motion-artwork] Lookup completed: no motion artwork available for this track");
                    }
                });
            } else {
                let _ = this.update(cx, |this, _| {
                    this.task = None;
                });
            }
        }));
    }
}
