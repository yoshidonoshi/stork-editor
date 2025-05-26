use crate::{engine::displayengine::DisplayEngine, gui::gui::StorkTheme};

pub fn stork_settings_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    ui.heading("Settings");
    let _cur_layer_combo = egui::ComboBox::from_label("Theme")
        .selected_text(format!("{}",de.display_settings.stork_theme))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut de.display_settings.stork_theme, StorkTheme::AUTO, StorkTheme::AUTO.to_string());
            ui.selectable_value(&mut de.display_settings.stork_theme, StorkTheme::LIGHT, StorkTheme::LIGHT.to_string());
            ui.selectable_value(&mut de.display_settings.stork_theme, StorkTheme::DARK, StorkTheme::DARK.to_string());
        });
    let sys_theme = ui.ctx().system_theme().unwrap_or(egui::Theme::Dark);
    ui.ctx().set_theme(match de.display_settings.stork_theme {
        StorkTheme::DARK => egui::Theme::Dark,
        StorkTheme::LIGHT => egui::Theme::Light,
        StorkTheme::AUTO => sys_theme,
    });
    // TODO: Sticky backgrounds. Check for primary layers with 256
    // Sprite Graphics Render Mode
    let show_cb = egui::Checkbox::new(&mut de.display_settings.show_box_for_rendered, "Show true position of rendered Sprites");
    ui.add(show_cb);
}