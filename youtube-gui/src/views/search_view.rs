use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::{DurationFilter, SearchFilter, SearchOrder, SearchType, Video};

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::{draw_video_card, help_badge, render_error_banner};

pub struct SearchFilterState {
    pub order: SearchOrder,
    pub duration: DurationFilter,
    pub content_type: SearchType,
    pub upload_date: UploadDateFilter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadDateFilter {
    AnyTime,
    Today,
    ThisWeek,
    ThisMonth,
    ThisYear,
}

impl UploadDateFilter {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AnyTime => "Any Time",
            Self::Today => "Today (24h)",
            Self::ThisWeek => "This Week",
            Self::ThisMonth => "This Month",
            Self::ThisYear => "This Year",
        }
    }

    pub fn to_rfc3339(self) -> Option<String> {
        let secs_ago: u64 = match self {
            Self::AnyTime => return None,
            Self::Today => 86_400,
            Self::ThisWeek => 7 * 86_400,
            Self::ThisMonth => 30 * 86_400,
            Self::ThisYear => 365 * 86_400,
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let target = now.saturating_sub(secs_ago);
        Some(format_unix_to_rfc3339(target))
    }
}

fn format_unix_to_rfc3339(total_secs: u64) -> String {
    let sec = (total_secs % 60) as u32;
    let total_mins = total_secs / 60;
    let min = (total_mins % 60) as u32;
    let total_hours = total_mins / 60;
    let hour = (total_hours % 24) as u32;
    let mut days = (total_hours / 24) as i64;

    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

impl Default for SearchFilterState {
    fn default() -> Self {
        Self {
            order: SearchOrder::Relevance,
            duration: DurationFilter::Any,
            content_type: SearchType::Video,
            upload_date: UploadDateFilter::AnyTime,
        }
    }
}

impl SearchFilterState {
    pub fn to_search_filter(&self) -> SearchFilter {
        SearchFilter {
            order: Some(self.order),
            duration: match self.duration {
                DurationFilter::Any => None,
                d => Some(d),
            },
            content_type: Some(self.content_type),
            published_after: self.upload_date.to_rfc3339(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_search_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    search_query: &mut String,
    filter_state: &mut SearchFilterState,
    results: &Option<Result<Arc<Vec<Video>>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    let recent_searches: Vec<String> = {
        let s = state.lock().unwrap();
        s.recent_searches.iter().cloned().collect()
    };

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("🔍 Search YouTube")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.add_space(16.0);
        let response = ui.add(
            egui::TextEdit::singleline(search_query)
                .hint_text("Search videos, channels, topics...")
                .desired_width(300.0),
        );
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let q = search_query.trim().to_string();
            if !q.is_empty() {
                action = Some(PendingAction::Search {
                    query: q,
                    filter: Some(filter_state.to_search_filter()),
                });
            }
        }
        if !search_query.is_empty()
            && ui
                .button("✖")
                .on_hover_text("Clear search input")
                .clicked()
        {
            search_query.clear();
        }
        let can_search = !search_query.trim().is_empty();
        if ui
            .add_enabled(can_search, egui::Button::new("🔍 Search"))
            .on_hover_text("Run YouTube search (100 quota units)")
            .on_disabled_hover_text("Type a search term first")
            .clicked()
        {
            let q = search_query.trim().to_string();
            action = Some(PendingAction::Search {
                query: q,
                filter: Some(filter_state.to_search_filter()),
            });
        }
        help_badge(
            ui,
            "Each YouTube search costs 100 API quota units. Use recent search chips or Subscriptions/New Videos to conserve quota.",
        );
    });

    // Recent Searches bar (Heuristic 6: Recognition Rather Than Recall)
    if !recent_searches.is_empty() {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("🕒 Recent:")
                    .size(12.0)
                    .color(egui::Color32::GRAY),
            );
            for recent in &recent_searches {
                if ui
                    .small_button(recent)
                    .on_hover_text(format!("Search again for \"{recent}\""))
                    .clicked()
                {
                    *search_query = recent.clone();
                    action = Some(PendingAction::Search {
                        query: recent.clone(),
                        filter: Some(filter_state.to_search_filter()),
                    });
                }
            }
            if ui
                .small_button("✖ Clear History")
                .on_hover_text("Clear recent search chips")
                .clicked()
            {
                action = Some(PendingAction::ClearRecentSearches);
            }
        });
    }

    ui.add_space(8.0);

    // Search Filter Toolbar (Sort, Duration, Content Type, Upload Date)
    let mut filter_changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("🎛 Filters:").strong());
        ui.add_space(6.0);

        ui.label("Sort:");
        egui::ComboBox::from_id_source("search_order_cb")
            .selected_text(filter_state.order.label())
            .width(120.0)
            .show_ui(ui, |ui| {
                let orders = [
                    SearchOrder::Relevance,
                    SearchOrder::Date,
                    SearchOrder::ViewCount,
                    SearchOrder::Rating,
                    SearchOrder::Title,
                ];
                for o in orders {
                    if ui
                        .selectable_value(&mut filter_state.order, o, o.label())
                        .changed()
                    {
                        filter_changed = true;
                    }
                }
            });

        ui.add_space(8.0);
        ui.label("Duration:");
        egui::ComboBox::from_id_source("search_duration_cb")
            .selected_text(filter_state.duration.label())
            .width(140.0)
            .show_ui(ui, |ui| {
                let durations = [
                    DurationFilter::Any,
                    DurationFilter::Short,
                    DurationFilter::Medium,
                    DurationFilter::Long,
                ];
                for d in durations {
                    if ui
                        .selectable_value(&mut filter_state.duration, d, d.label())
                        .changed()
                    {
                        filter_changed = true;
                    }
                }
            });

        ui.add_space(8.0);
        ui.label("Type:");
        egui::ComboBox::from_id_source("search_type_cb")
            .selected_text(filter_state.content_type.label())
            .width(100.0)
            .show_ui(ui, |ui| {
                let types = [
                    SearchType::Video,
                    SearchType::Channel,
                    SearchType::Playlist,
                    SearchType::All,
                ];
                for t in types {
                    if ui
                        .selectable_value(&mut filter_state.content_type, t, t.label())
                        .changed()
                    {
                        filter_changed = true;
                    }
                }
            });

        ui.add_space(8.0);
        ui.label("Date:");
        egui::ComboBox::from_id_source("search_date_cb")
            .selected_text(filter_state.upload_date.label())
            .width(115.0)
            .show_ui(ui, |ui| {
                let dates = [
                    UploadDateFilter::AnyTime,
                    UploadDateFilter::Today,
                    UploadDateFilter::ThisWeek,
                    UploadDateFilter::ThisMonth,
                    UploadDateFilter::ThisYear,
                ];
                for dt in dates {
                    if ui
                        .selectable_value(&mut filter_state.upload_date, dt, dt.label())
                        .changed()
                    {
                        filter_changed = true;
                    }
                }
            });

        if filter_state.order != SearchOrder::Relevance
            || filter_state.duration != DurationFilter::Any
            || filter_state.content_type != SearchType::Video
            || filter_state.upload_date != UploadDateFilter::AnyTime
        {
            ui.add_space(6.0);
            if ui
                .button("↺ Reset Filters")
                .on_hover_text("Reset all search filters to default")
                .clicked()
            {
                *filter_state = SearchFilterState::default();
                filter_changed = true;
            }
        }
    });

    if filter_changed {
        let q = search_query.trim().to_string();
        if !q.is_empty() {
            action = Some(PendingAction::Search {
                query: q,
                filter: Some(filter_state.to_search_filter()),
            });
        }
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    match results {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                if search_query.trim().is_empty() {
                    ui.label("Enter a search term above and press Search or Enter.");
                } else {
                    ui.spinner();
                    ui.add_space(10.0);
                    ui.label(format!(
                        "Searching YouTube for \"{}\"...",
                        search_query.trim()
                    ));
                }
            });
        }
        Some(Err(err_msg)) => {
            let mut banner_action = PendingAction::None;
            render_error_banner(
                ui,
                "Search failed",
                err_msg,
                PendingAction::RetrySearch {
                    query: search_query.trim().to_string(),
                    filter: Some(filter_state.to_search_filter()),
                },
                &mut banner_action,
            );
            if !matches!(banner_action, PendingAction::None) {
                action = Some(banner_action);
            }
        }
        Some(Ok(vids)) => {
            if vids.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No search results found. Try broadening your filters or keywords.");
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for video in vids.iter() {
                            let mut card_action = PendingAction::None;
                            draw_video_card(
                                state,
                                http_client,
                                audio_tx,
                                ui,
                                ctx,
                                video,
                                &mut card_action,
                            );
                            if !matches!(card_action, PendingAction::None) {
                                action = Some(card_action);
                            }
                        }
                    });
            }
        }
    }

    action
}
