use crate::{engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

#[derive(Default)]
pub struct ResizeSettings {
    pub new_width: u16,
    pub new_height: u16,
    pub reset_needed: bool,
    pub window_open: bool
}

pub fn show_resize_modal(ui: &mut egui::Ui, de: &mut DisplayEngine, settings: &mut ResizeSettings) {
    if !de.display_settings.is_cur_layer_bg() {
        log_write("Cannot resize, not on BG layer", LogLevel::WARN);
        settings.window_open = false;
        return;
    }
    if settings.reset_needed {
        let bg = de.loaded_map.get_background(de.display_settings.current_layer as u8);
        if bg.is_none() {
            log_write("Failed to get BG in resize modal", LogLevel::ERROR);
            settings.window_open = false;
            return;
        }
        let info = bg.unwrap().get_info();
        if info.is_none() {
            log_write("Failed to get INFO in resize modal", LogLevel::ERROR);
            settings.window_open = false;
            return;
        }
        let info = info.unwrap();
        settings.new_width = info.layer_width;
        settings.new_height = info.layer_height;
        settings.reset_needed = false;
    }
    ui.heading("Resize Current Layer");
    ui.label("Set the current layer's new width and height (must be even)");
    ui.label("WARNING: This action may be destructive");
    ui.horizontal(|ui| {
        let width = egui::DragValue::new(&mut settings.new_width)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        ui.add(width);
        ui.label("Width")
    });
    ui.horizontal(|ui| {
        let height = egui::DragValue::new(&mut settings.new_height)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        ui.add(height);
        ui.label("Height");
    });
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let button_cancel = ui.button("Cancel");
        if button_cancel.clicked() {
            // No update
            settings.reset_needed = true;
            settings.window_open = false;
        }
        let button_ok = ui.button("Okay");
        if button_ok.clicked() {
            // Do update
            log_write("Starting Layer resize...", LogLevel::DEBUG);
            let bg = de.loaded_map.get_background(de.display_settings.current_layer as u8);
            if bg.is_none() {
                log_write("Failed to get BG in resize modal resizing", LogLevel::ERROR);
                settings.window_open = false;
                return;
            }
            let bg = bg.unwrap();
            let info = bg.get_info_mut();
            if info.is_none() {
                log_write("Failed to get INFO in resize modal resizing", LogLevel::ERROR);
                settings.window_open = false;
                return;
            }
            let info = info.unwrap();
            log_write(format!("Changing size of layer from 0x{:X}/0x{:X} to 0x{:X}/0x{:X}",
                info.layer_width,info.layer_height,
                settings.new_width,settings.new_height), LogLevel::LOG);
            if settings.new_width % 2 != 0 {
                settings.new_width += 1;
            }
            if settings.new_height % 2 != 0 {
                settings.new_height += 1;
            }
            let increase_result = bg.increase_width(settings.new_width);
            if increase_result.is_err() {
                log_write("Error increasing size of layer", LogLevel::ERROR);
                settings.reset_needed = true;
                settings.window_open = false;
                return;
            }
            if increase_result.unwrap() != settings.new_width {
                log_write("Mismatch in result width", LogLevel::ERROR);
            } else {
                log_write("Resize successful, updating", LogLevel::LOG);
            }
            // Do things to trigger updates
            de.unsaved_changes = true;
            de.graphics_update_needed = true;
            settings.reset_needed = true;
            settings.window_open = false;
        }
    });
}