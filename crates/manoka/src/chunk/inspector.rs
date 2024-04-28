use bevy_inspector_egui::{
  egui, inspector_egui_impls::InspectorPrimitive,
  reflect_inspector::InspectorUi,
};

use super::Chunk;

const CHUNK_INSPECTOR_MESSAGE: &str =
  "I don't really know how to debug it right now, so I made this to avoid \
   crashing because of the default inspector implementation.";

fn debug_chunk_asset(ui: &mut egui::Ui) {
  ui.label("You found a Chunk asset!");
  ui.weak(CHUNK_INSPECTOR_MESSAGE);
}

impl InspectorPrimitive for Chunk {
  fn ui(
    &mut self,
    ui: &mut egui::Ui,
    _options: &dyn std::any::Any,
    _id: egui::Id,
    _env: InspectorUi<'_, '_>,
  ) -> bool {
    debug_chunk_asset(ui);
    false
  }

  fn ui_readonly(
    &self,
    ui: &mut egui::Ui,
    _options: &dyn std::any::Any,
    _id: egui::Id,
    _env: InspectorUi<'_, '_>,
  ) {
    debug_chunk_asset(ui);
  }
}
