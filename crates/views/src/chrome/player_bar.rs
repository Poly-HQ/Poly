// SPDX-License-Identifier: GPL-3.0-or-later
// Portions Copyright (C) Sonora Contributors
// Modifications Copyright (C) 2026 Poly Project Contributors

use router::{Destination, Link};
use std::time::{Duration, Instant};
use ui::ActiveTheme as _;

use gpui::prelude::*;
use gpui::{
    ClickEvent, Context, Entity, FontWeight, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Render, ScrollWheelEvent, SharedString, Task, Window, div, px,
};
use i18n::t;
use input::{ToggleFullscreen, ToggleLyrics, ToggleQueue};
use state::{AppSettings, Playback, Queue, SideTab, Sonora};
use ui::{
    Artwork, Button, ExplicitBadge, Popup, Room, Scrollbar, Scrubber, ScrubberState, clock,
    tabular,
};

use crate::shared::menus::ItemMenu;
use crate::shared::transport::{NOTCH, like, moved, percent, transport, volume_icon};

const VOLUME_TIGHT: f32 = 72.;

pub(crate) struct PlayerBar {
    playback: Entity<Playback>,
    queue: Entity<Queue>,
    settings: Entity<AppSettings>,
    track_menu: ItemMenu,
    context_menu: Option<(music::Track, Point<Pixels>)>,
    seek: ScrubberState,
    volume: ScrubberState,
    pending: Option<f32>,
    over_seek: Option<f32>,
    over_volume: Option<f32>,
    volume_held: bool,
    muted: Option<f32>,
    marquee: Option<(String, Instant)>,
    marquee_task: Option<Task<()>>,
}

impl PlayerBar {
    pub fn new(playback: Entity<Playback>, queue: Entity<Queue>, cx: &mut Context<Self>) -> Self {
        let library = Sonora::global(cx).library.clone();
        let settings = Sonora::global(cx).settings.clone();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        cx.observe(&queue, |_, _, cx| cx.notify()).detach();
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        cx.observe(&settings, |_, _, cx| cx.notify()).detach();

        let me = cx.entity_id();
        let playlist_scrollbar = cx.new(|_| Scrollbar::inset().watching(me));

        Self {
            playback,
            queue,
            settings,
            track_menu: ItemMenu::new(playlist_scrollbar, cx),
            context_menu: None,
            seek: ScrubberState::new("seek"),
            volume: ScrubberState::new("volume"),
            pending: None,
            over_seek: None,
            over_volume: None,
            volume_held: false,
            muted: None,
            marquee: None,
            marquee_task: None,
        }
    }

    fn open_context_menu(
        &mut self,
        track: music::Track,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.track_menu.reset(cx);
        self.context_menu = Some((track, position));
        cx.notify();
    }

    /// Opens the track menu on whatever is playing when the click lands, so a render does not
    /// have to hold a copy of the track for a menu that is rarely asked for.
    fn open_playing_menu(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(track) = self.playback.read(cx).track().cloned() else {
            return;
        };
        self.open_context_menu(track, position, cx);
    }

    fn commit_seek(&mut self, cx: &mut Context<Self>) {
        let Some(fraction) = self.pending.take() else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.seek_fraction(fraction, cx));
    }

    fn hover(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let pad = cx.theme().metrics.pad;
        let seek = self.seek.hovered(event.position, pad);
        let volume = self.volume.hovered(event.position, pad);

        if moved(self.over_seek, seek) || moved(self.over_volume, volume) {
            self.over_seek = seek;
            self.over_volume = volume;
            cx.notify();
        }
    }

    fn turn_volume(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(window.line_height()).y;
        if delta == Pixels::ZERO {
            return;
        }
        cx.stop_propagation();

        let notch = match delta > Pixels::ZERO {
            true => NOTCH,
            false => -NOTCH,
        };
        let level = (self.playback.read(cx).volume() + notch).clamp(0., 1.);
        self.muted = None;
        self.playback
            .update(cx, |playback, cx| playback.set_volume(level, cx));
    }

    fn sound(&self, width: Pixels, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let empty = theme.muted_foreground.opacity(0.3);
        let level = self.playback.read(cx).volume();
        let showing = self.over_volume.is_some() || self.volume_held;
        let bubble = showing.then(|| (level, percent(level)));
        let restore = self.muted.unwrap_or(0.7);

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .on_scroll_wheel(cx.listener(Self::turn_volume))
            .child(
                Button::new("volume")
                    .ghost()
                    .small()
                    .icon(volume_icon(level))
                    .tooltip_above(match level <= 0.001 {
                        true => "player-unmute",
                        false => "player-mute",
                    })
                    .tint(theme.muted_foreground)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let wanted = match level <= 0.001 {
                            true => restore,
                            false => 0.,
                        };
                        this.muted = match wanted {
                            0. => Some(level),
                            _ => None,
                        };
                        this.playback
                            .update(cx, |playback, cx| playback.set_volume(wanted, cx));
                    })),
            )
            .child(
                div().w(width).flex_none().child(
                    Scrubber::new(&self.volume, level)
                        .colors(theme.progress_bar, empty, theme.foreground)
                        .when_some(bubble, |this, (at, text)| this.bubble(at, text))
                        .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                            let level = *fraction;
                            this.volume_held = true;
                            this.muted = None;
                            this.playback
                                .update(cx, |playback, cx| playback.set_volume(level, cx));
                        }))
                        .on_release(cx.listener(|this, _: &MouseUpEvent, _, cx| {
                            this.volume_held = false;
                            cx.notify();
                        })),
                ),
            )
    }

    fn fullscreen_button(&self) -> Button {
        Button::new("toggle-fullscreen")
            .ghost()
            .small()
            .icon("icons/maximize.svg")
            .tooltip_above("player-fullscreen")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleFullscreen), cx))
    }
}

impl PlayerBar {
    pub(crate) fn height(window: &Window, _cx: &gpui::App) -> Pixels {
        let span = Room::of(window.viewport_size().width);
        match !span.fits(Room::Roomy) {
            true => ui::snapped(px(112.), window),
            false => ui::snapped(px(88.), window),
        }
    }
}

impl Render for PlayerBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let empty = muted.opacity(0.3);
        let span = Room::of(window.viewport_size().width);
        let stacked = !span.fits(Room::Roomy);
        let height = Self::height(window, cx);
        let clock_text = theme.text(ui::Text::Tiny);

        let playback = self.playback.read(cx);
        let seekable = playback.track().is_some();
        let progress = self.pending.unwrap_or_else(|| playback.progress());
        let elapsed = playback.position();
        let total = playback
            .track()
            .map(|track| track.duration)
            .unwrap_or(Duration::ZERO);
        let seek_bubble = self
            .over_seek
            .or(self.pending)
            .map(|at| (at, clock(total.mul_f32(at))));

        let seek = div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .child(
                div().flex_1().min_w_0().child(
                    Scrubber::new(&self.seek, progress)
                        .colors(theme.progress_bar, empty, theme.foreground)
                        .enabled(seekable)
                        .when_some(seek_bubble, |this, (at, text)| this.bubble(at, text))
                        .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                            this.pending = Some(*fraction);
                            cx.notify();
                        }))
                        .on_release(
                            cx.listener(|this, _: &MouseUpEvent, _, cx| this.commit_seek(cx)),
                        ),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_none()
                    .whitespace_nowrap()
                    .text_size(clock_text)
                    .font_features(tabular())
                    .text_color(muted)
                    .child(clock(elapsed))
                    .child(div().child("/").text_color(muted.opacity(0.4)))
                    .child(clock(total)),
            );

        let track = playback.track();
        let cover = track.and_then(|track| track.cover.clone());
        let explicit = track.is_some_and(|track| track.explicit);
        let like = like(track, &self.playback, cx);

        let left_section = div()
            .flex()
            .items_center()
            .gap(px(16.))
            .flex_1()
            .min_w_0()
            .pr(px(16.))
            .child(
                div()
                    .id("now-playing-artwork")
                    .size(px(56.))
                    .flex_none()
                    .overflow_hidden()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(theme.border.opacity(0.4))
                    .when_some(
                        track.and_then(|track| track.album_id.clone()),
                        |this, album| this.link(Destination::Album(album.into())),
                    )
                    .when(track.is_some(), |this| {
                        this.on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                window.prevent_default();
                                this.open_playing_menu(event.position, cx);
                            }),
                        )
                    })
                    .child(Artwork::new(cover).size(px(56.))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap_1()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(match track {
                                Some(t) => {
                                    let author_refs = match !t.artist_refs.is_empty() {
                                        true => t.artist_refs.clone(),
                                        false => t
                                            .artists
                                            .split(", ")
                                            .filter(|s| !s.is_empty())
                                            .map(|name| music::ArtistRef {
                                                name: name.to_string(),
                                                id: None,
                                            })
                                            .collect(),
                                    };
                                    let full_key = format!("{} {}", t.name, t.artists);
                                    let max_width_f32 = match span {
                                        Room::Tight | Room::Snug => 200.,
                                        Room::Roomy => 260.,
                                        Room::Wide => 340.,
                                        Room::Vast => 440.,
                                    };
                                    let max_width = px(max_width_f32);

                                    let name_width = estimate_player_text_width(&t.name, true);
                                    let artists_width = estimate_player_text_width(&t.artists, false);
                                    let unit_width = name_width + 8.0 + artists_width;
                                    let is_long = unit_width > max_width_f32;
                                    let gap = 48.0;
                                    let total_span = unit_width + gap;
                                    let mut offset = Pixels::ZERO;

                                    if is_long {
                                        let now = Instant::now();
                                        let start = match &mut self.marquee {
                                            Some((key, start)) if *key == full_key => *start,
                                            slot => {
                                                self.marquee_task = None;
                                                *slot = Some((full_key.clone(), now));
                                                now
                                            }
                                        };
                                        let elapsed = now.duration_since(start).as_secs_f32();
                                        let pause_duration = 5.0;
                                        let speed = 28.0;
                                        let slide_duration = (total_span / speed).max(4.0);
                                        let cycle_duration = pause_duration + slide_duration;
                                        let cycle = elapsed % cycle_duration;

                                        if cycle < pause_duration {
                                            offset = Pixels::ZERO;
                                            if self.marquee_task.is_none() {
                                                let delay = Duration::from_secs_f32(
                                                    (pause_duration - cycle).max(0.02) + 0.02,
                                                );
                                                self.marquee_task = Some(cx.spawn(async move |this, cx| {
                                                    cx.background_executor().timer(delay).await;
                                                    let _ = this.update(cx, |this, cx| {
                                                        this.marquee_task = None;
                                                        cx.notify();
                                                    });
                                                }));
                                            }
                                        } else {
                                            self.marquee_task = None;
                                            let progress = ((cycle - pause_duration) / slide_duration)
                                                .clamp(0.0, 1.0);
                                            offset = -px(total_span * progress);
                                            window.request_animation_frame();
                                        }
                                    } else {
                                        self.marquee = None;
                                        self.marquee_task = None;
                                    }

                                    let make_unit = |prefix: &'static str| {
                                        let mut authors_row = div().flex().items_center().gap_1();
                                        for (i, artist) in author_refs.iter().enumerate() {
                                            if i > 0 {
                                                authors_row = authors_row.child(
                                                    div()
                                                        .child(",")
                                                        .text_color(theme.muted_foreground)
                                                        .text_size(theme.text(ui::Text::Body)),
                                                );
                                            }
                                            let mut link_item = div()
                                                .id((prefix, i))
                                                .child(SharedString::from(artist.name.clone()))
                                                .text_color(theme.muted_foreground)
                                                .text_size(theme.text(ui::Text::Body))
                                                .whitespace_nowrap();
                                            if let Some(id) = &artist.id {
                                                link_item = link_item
                                                    .cursor_pointer()
                                                    .hover(|style| {
                                                        style.text_color(theme.foreground).underline()
                                                    })
                                                    .link(Destination::Artist(id.clone().into()));
                                            }
                                            authors_row = authors_row.child(link_item);
                                        }

                                        div()
                                            .flex()
                                            .flex_none()
                                            .when(is_long, |this| this.w(px(unit_width)))
                                            .items_center()
                                            .gap_2()
                                            .whitespace_nowrap()
                                            .child(
                                                div()
                                                    .child(SharedString::from(t.name.clone()))
                                                    .text_size(theme.text(ui::Text::Body))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(theme.foreground),
                                            )
                                            .child(authors_row)
                                    };

                                    let content = if is_long {
                                        div()
                                            .id("now-playing-track-content")
                                            .flex()
                                            .items_center()
                                            .gap(px(gap))
                                            .whitespace_nowrap()
                                            .relative()
                                            .left(offset)
                                            .child(make_unit("np-a"))
                                            .child(make_unit("np-b"))
                                    } else {
                                        div()
                                            .id("now-playing-track-content")
                                            .flex()
                                            .items_center()
                                            .whitespace_nowrap()
                                            .child(make_unit("np-a"))
                                    };

                                    div()
                                        .id("now-playing-track-marquee-container")
                                        .overflow_hidden()
                                        .when(is_long, |this| this.max_w(max_width))
                                        .child(content)
                                }
                                None => {
                                    self.marquee = None;
                                    self.marquee_task = None;
                                    div()
                                        .id("now-playing-idle")
                                        .child(t!("player-nothing-playing"))
                                        .text_size(theme.text(ui::Text::Body))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(muted)
                                        .truncate()
                                }
                            })
                            .when(explicit, |this| {
                                this.child(div().flex_none().child(ExplicitBadge::new()))
                            })
                            .child(like),
                    )
                    .child(seek),
            );

        let (open, tab) = {
            let settings = self.settings.read(cx);
            (settings.sidebar_right_open(), settings.sidebar_right_tab())
        };
        let lyrics_active = open && tab == SideTab::Lyrics;
        let queue_active = open && tab == SideTab::Queue;

        let right_section = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(10.))
            .child(transport(&self.playback, &self.queue, false, cx))
            .child(
                Button::new("player-lyrics")
                    .ghost()
                    .small()
                    .icon("icons/mic-2.svg")
                    .tooltip_above("lyrics-title")
                    .selected(lyrics_active)
                    .tint(match lyrics_active {
                        true => theme.foreground,
                        false => theme.muted_foreground,
                    })
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(ToggleLyrics), cx);
                    }),
            )
            .child(
                Button::new("player-queue")
                    .ghost()
                    .small()
                    .icon("icons/list-music.svg")
                    .tooltip_above("queue-title")
                    .selected(queue_active)
                    .tint(match queue_active {
                        true => theme.foreground,
                        false => theme.muted_foreground,
                    })
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(ToggleQueue), cx);
                    }),
            )
            .child(
                Button::new("player-more")
                    .ghost()
                    .small()
                    .icon("icons/more-horizontal.svg")
                    .tooltip_above("menu-more")
                    .tint(muted)
                    .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                        this.open_playing_menu(event.position(), cx);
                    })),
            )
            .child(self.sound(px(VOLUME_TIGHT), cx))
            .child(self.fullscreen_button());

        let ambient = crate::shared::ambient::shown(cx);

        let base = div()
            .flex()
            .w_full()
            .h(height)
            .flex_none()
            .px(px(20.))
            .rounded(px(16.))
            .border_1()
            .border_color(theme.border)
            .when_else(
                ambient || ui::blurring(cx),
                |this| ui::glass(this, cx),
                |this| this.bg(theme.secondary),
            )
            .shadow_lg()
            .on_mouse_move(cx.listener(Self::hover));

        let context_menu = self.context_menu.clone().map(|(track, position)| {
            Popup::new(position, self.track_menu.for_track(&track, cx)).on_close(cx.listener(
                |this, _, _, cx| {
                    this.context_menu = None;
                    cx.notify();
                },
            ))
        });

        let content = match stacked {
            true => base
                .flex_col()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .child(left_section)
                        .child(transport(&self.playback, &self.queue, false, cx)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_2()
                        .w_full()
                        .child(self.sound(px(VOLUME_TIGHT), cx))
                        .child(self.fullscreen_button()),
                ),
            false => base
                .items_center()
                .justify_between()
                .child(left_section)
                .child(right_section),
        };

        content.when_some(context_menu, |this, menu| this.child(menu))
    }
}

fn estimate_player_text_width(text: &str, is_bold: bool) -> f32 {
    let mut width = 0.0;
    for c in text.chars() {
        let u = c as u32;
        if (0x0E31..=0x0E3A).contains(&u) || (0x0E47..=0x0E4E).contains(&u) {
            continue;
        }
        if c.is_ascii_whitespace() {
            width += 4.0;
        } else if c.is_ascii_punctuation() {
            width += 4.5;
        } else if c.is_ascii() {
            width += if is_bold { 8.0 } else { 7.2 };
        } else if (0x0E00..=0x0E7F).contains(&u) {
            width += if is_bold { 10.0 } else { 9.0 };
        } else if (0x3000..=0x9FFF).contains(&u) || (0xAC00..=0xD7AF).contains(&u) {
            width += 14.0;
        } else {
            width += 8.5;
        }
    }
    width
}
