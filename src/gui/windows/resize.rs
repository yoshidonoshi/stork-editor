use egui::Color32;

use crate::{engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}, NON_MAIN_FOCUSED};

#[derive(Default)]
pub struct ResizeSettings {
    pub new_width: u16,
    pub new_height: u16,
    pub reset_needed: bool,
    pub window_open: bool
}

pub fn show_resize_modal(ui: &mut egui::Ui, de: &mut DisplayEngine, settings: &mut ResizeSettings) {
    puffin::profile_function!();
    if !de.display_settings.is_cur_layer_bg() {
        log_write("Cannot resize, not on BG layer", LogLevel::Warn);
        settings.window_open = false;
        return;
    }
    // Get BG and INFO read-only
    let Some(bg) = de.loaded_map.get_background(de.display_settings.current_layer as u8) else {
        log_write("Failed to get BG in resize modal", LogLevel::Error);
        settings.window_open = false;
        return;
    };
    let Some(info) = bg.get_info() else {
        log_write("Failed to get INFO in resize modal", LogLevel::Error);
        settings.window_open = false;
        return;
    };
    // Check reset
    let mut okay_enabled = true;
    if settings.reset_needed {
        settings.new_width = info.layer_width;
        settings.new_height = info.layer_height;
        settings.reset_needed = false;
    }
    ui.heading("Resize Current Layer");
    ui.label("Width and height must both be even numbers");
    ui.label(format!("Current Width and Height: 0x{:X}/0x{:X}",info.layer_width,info.layer_height));
    if settings.new_height < info.layer_height || settings.new_width < info.layer_width {
        ui.label(egui::RichText::new("Warning: this action is highly destructive").color(Color32::RED));
    } else {
        ui.label(" ");
    }
    ui.horizontal(|ui| {
        let width = egui::DragValue::new(&mut settings.new_width)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        let wres = ui.add(width);
        if wres.has_focus() {
            *NON_MAIN_FOCUSED.lock().unwrap() = true;
        }
        ui.label("Width")
    });
    ui.horizontal(|ui| {
        let height = egui::DragValue::new(&mut settings.new_height)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        let lres = ui.add(height);
        if lres.has_focus() {
            *NON_MAIN_FOCUSED.lock().unwrap() = true;
        }
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
        // No odd values
        if settings.new_height % 2 != 0 {
            okay_enabled = false;
        }
        if settings.new_width % 2 != 0 {
            okay_enabled = false;
        }
        let button_ok = ui.add_enabled(okay_enabled, egui::Button::new("Okay"));
        if button_ok.clicked() {
            // Do update with mutable versions
            let Some(bg) = de.loaded_map.get_background(de.display_settings.current_layer as u8) else {
                log_write("Failed to get BG in resize modal resizing", LogLevel::Error);
                settings.window_open = false;
                return;
            };
            let Some(info) = bg.get_info_mut() else {
                log_write("Failed to get INFO in resize modal resizing", LogLevel::Error);
                settings.window_open = false;
                return;
            };
            log_write(format!("Changing size of layer from 0x{:X}/0x{:X} to 0x{:X}/0x{:X}",
                info.layer_width,info.layer_height,
                settings.new_width,settings.new_height), LogLevel::Log);
            // Actual resizing calls
            if settings.new_width > info.layer_width {
                // Width is greater, increase width //
                let Ok(increase_result) = bg.increase_width(settings.new_width) else {
                    log_write("Error increasing size of layer", LogLevel::Error);
                    settings.reset_needed = true;
                    settings.window_open = false;
                    return;
                };
                if increase_result != settings.new_width {
                    log_write("Mismatch in result width", LogLevel::Error);
                } else {
                    log_write("Resize successful, updating", LogLevel::Log);
                }
            } else if settings.new_width < info.layer_width {
                let Ok(decrease_result) = bg.decrease_width(settings.new_width) else {
                    log_write("Error decreasing size of layer", LogLevel::Error);
                    settings.reset_needed = true;
                    settings.window_open = false;
                    return;
                };
                if decrease_result != settings.new_width {
                    log_write("Mismatch in result width", LogLevel::Error);
                } else {
                    log_write("Resize successful, updating", LogLevel::Log);
                }
            } else {
                log_write("No change in layer width", LogLevel::Debug);
            }
            if bg.change_height(settings.new_height).is_err() {
                log_write("Error changing height of layer", LogLevel::Error);
                settings.reset_needed = true;
                settings.window_open = false;
                return;
            }
            // Trim sprites
            let Some(spr) = de.loaded_map.get_setd() else {
                log_write("Failed to get SETD when resizing", LogLevel::Fatal);
                unreachable!()
            };
            let trimmed = spr.trim(settings.new_width, settings.new_height);
            log_write(format!("Trimmed {} Sprites on resize",trimmed), LogLevel::Debug);
            // Do things to trigger updates
            log_write("graphics updated", LogLevel::Debug);
            de.unsaved_changes = true;
            de.graphics_update_needed = true;
            settings.reset_needed = true;
            settings.window_open = false;
        }
    });
}
