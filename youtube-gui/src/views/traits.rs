use eframe::egui;
use crate::types::{AppState, PendingAction};

pub trait GuiView {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, state: &AppState) -> Option<PendingAction>;
}
