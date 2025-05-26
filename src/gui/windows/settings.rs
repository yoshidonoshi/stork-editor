use crate::{engine::displayengine::DisplayEngine, gui::gui::{StorkSettings, StorkTheme}};

pub fn stork_settings_window(ui: &mut egui::Ui, _de: &mut DisplayEngine, settings: &mut StorkSettings) {
    ui.heading("Settings");
    let _cur_layer_combo = egui::ComboBox::from_label("Theme")
        .selected_text(format!("{}",settings.theme))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut settings.theme, StorkTheme::AUTO, StorkTheme::AUTO.to_string());
            ui.selectable_value(&mut settings.theme, StorkTheme::LIGHT, StorkTheme::LIGHT.to_string());
            ui.selectable_value(&mut settings.theme, StorkTheme::DARK, StorkTheme::DARK.to_string());
        });
    let sys_theme = ui.ctx().system_theme().unwrap_or(egui::Theme::Dark);
    ui.ctx().set_theme(match settings.theme {
        StorkTheme::DARK => egui::Theme::Dark,
        StorkTheme::LIGHT => egui::Theme::Light,
        StorkTheme::AUTO => sys_theme,
    });
    // TODO: Sticky backgrounds. Check for primary layers with 256
}