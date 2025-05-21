use egui::Color32;

use crate::{data::{backgrounddata::BackgroundData, mapfile::TopLevelSegmentWrapper, TopLevelSegment}, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

pub fn show_map_segments_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    let mut do_del: Option<usize> = Option::None;
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .min_scrolled_height(1.0)
        .show(ui, |ui| {
            for (i,seg) in &mut de.loaded_map.segments.iter_mut().enumerate() {
                let header = seg.header().clone();
                match header.as_str() {
                    "SCEN" => {
                        ui.heading("SCEN");
                        if let TopLevelSegmentWrapper::SCEN(scendata) = seg {
                            show_scen_data(ui, scendata);
                        } else {
                            ui.label("ERROR: Could not retrieve SCEN");
                        }
                    }
                    "ALPH" => {
                        ui.heading("ALPH");
                        if let TopLevelSegmentWrapper::ALPH(alph) = seg {
                            ui.label(format!("BLDALPHA: 0x{:X}",&alph.bldalpha));
                            ui.label(format!("BLDCNT: 0x{:X}",&alph.bldcnt));
                        } else {
                            ui.label("ERROR: Could not retrieve ALPH");
                        }
                    }
                    "GRAD" => {
                        ui.heading("GRAD");
                        if let TopLevelSegmentWrapper::GRAD(grad) = seg {
                            ui.label(format!("Color Count: 0x{:X}",grad.color_count));
                            ui.label(format!("Y Offset: 0x{:X}",grad.y_offset));
                        } else {
                            ui.label("ERROR: Could not retrieve GRAD");
                        }
                    }
                    "SETD" => {
                        ui.heading("SETD");
                        if let TopLevelSegmentWrapper::SETD(setd) = seg {
                            ui.label(format!("Sprite count: {}",setd.sprites.len()));
                        } else {
                            ui.label("ERROR: Could not retrieve SETD");
                        }
                    }
                    "BLKZ" => {
                        ui.heading("BLKZ");
                        if let TopLevelSegmentWrapper::BLKZ(blkz) = seg {
                            ui.label(format!("Height/Width: 0x{:04X}/0x{:04X}",&blkz.height,&blkz.width));
                            ui.label(format!("X/Y Offset: 0x{:04X}/0x{:04X}",&blkz.x_offset,&blkz.y_offset));
                        } else {
                            ui.label("ERROR: Could not retrieve BLKZ");
                        }
                    }
                    "BRAK" => {
                        ui.heading("BRAK");
                        if let TopLevelSegmentWrapper::BRAK(brak) = seg {
                            ui.label(format!("Size in bytes: 0x{:X}",brak.raw_bytes.len()));
                        } else {
                            ui.label("ERROR: Could not retrieve BRAK");
                        }
                    }
                    "AREA" => {
                        ui.heading("AREA");
                        if let TopLevelSegmentWrapper::AREA(area) = seg {
                            ui.label(format!("Trigger count: {}",area.triggers.len()));
                        } else {
                            ui.label("ERROR: Could not retrieve AREA");
                        }
                    }
                    "PATH" => {
                        ui.heading("PATH");
                        if let TopLevelSegmentWrapper::PATH(path) = seg {
                            ui.label(format!("Path count: {}",path.lines.len()));
                        } else {
                            ui.label("ERROR: Could not retrieve PATH");
                        }
                    }
                    _ => {
                        ui.label(format!("Unhandled: {}",seg.header()));
                    }
                }
                ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
                let is_undeletable = header.eq("SETD") || header.eq("SCEN");
                let del_button = ui.add_enabled(!is_undeletable, egui::Button::new("Delete"));
                if del_button.clicked() {
                    do_del = Some(i);
                }
                ui.separator();
            }
        });
    if let Some(to_del) = do_del {
        let header = &de.loaded_map.segments[to_del].header();
        log_write(format!("Deleting segment '{}' at index {}",header,to_del), LogLevel::LOG);
        // These are way too important, and can just be emptied instead of outright deleted
        match header.as_str() {
            "SETD" => {
                log_write("Cannot delete Sprite database", LogLevel::WARN);
                return;
            }
            "SCEN" => {
                log_write("Cannot delete Background", LogLevel::WARN);
                return;
            }
            _ => { /* Do nothing */ }
        }
        de.loaded_map.segments.remove(to_del);
        de.graphics_update_needed = true;
        de.unsaved_changes = true;
    }
}

fn show_scen_data(ui: &mut egui::Ui, scen: &mut BackgroundData) {
    let info = scen.get_info().expect("INFO is guaranteed");
    ui.label(format!("BG Index: {}",info.which_bg));
}
