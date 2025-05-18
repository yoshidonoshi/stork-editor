use super::gui::Gui;

pub fn side_panel_show(ui: &mut egui::Ui, gui_state: &mut Gui) {
    ui.toggle_value(&mut gui_state.palette_window_open, "Palettes");
    ui.toggle_value(&mut gui_state.tile_preview_window_open, "Tiles");
    ui.toggle_value(&mut gui_state.brush_window_open, "Brush");
    ui.toggle_value(&mut gui_state.stamps_window_open, "Saved Brushes");
    ui.toggle_value(&mut gui_state.collision_window_open, "Collision");
    ui.toggle_value(&mut gui_state.path_window_open, "Paths");
    ui.toggle_value(&mut gui_state.sprites_window_open, "Add Sprites");
    ui.toggle_value(&mut gui_state.course_window_open, "Course Settings");
    ui.toggle_value(&mut gui_state.area_window_open, "Triggers");
    ui.toggle_value(&mut gui_state.mpdz_window_open, "Map Data");
    ui.toggle_value(&mut gui_state.scen_window_open, "BG Data");
}