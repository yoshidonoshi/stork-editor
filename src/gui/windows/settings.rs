use strum::IntoEnumIterator;

use crate::{engine::displayengine::DisplayEngine, gui::gui::StorkTheme};

pub fn stork_settings_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    ui.heading("Settings");
    let _cur_layer_combo = egui::ComboBox::from_label("Theme")
        .selected_text(format!("{}",de.display_settings.stork_theme))
        .show_ui(ui, |ui| {
            for theme in StorkTheme::iter() {
                ui.selectable_value(&mut de.display_settings.stork_theme, theme, theme.to_string());
            }
        });
    let sys_theme = ui.ctx().system_theme().unwrap_or(egui::Theme::Dark);
    ui.ctx().set_theme(match de.display_settings.stork_theme {
        StorkTheme::Dark => egui::Theme::Dark,
        StorkTheme::Light => egui::Theme::Light,
        StorkTheme::Auto => sys_theme,
    });
    // TODO: Sticky backgrounds. Check for primary layers with 256
    // Sprite Graphics Render Mode
    let show_cb = egui::Checkbox::new(&mut de.display_settings.show_box_for_rendered, "Show true position of rendered Sprites");
    ui.add(show_cb);
}