use crate::{data::{backgrounddata::BackgroundData, mapfile::TopLevelSegmentWrapper, TopLevelSegment}, engine::displayengine::DisplayEngine};

pub fn show_map_segments_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .min_scrolled_height(1.0)
        .show(ui, |ui| {
            for seg in &mut de.loaded_map.segments {
                match seg.header().as_str() {
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
                    _ => {
                        ui.label(format!("Unhandled: {}",seg.header()));
                    }
                }
                ui.separator();
            }
        });
}

fn show_scen_data(ui: &mut egui::Ui, scen: &mut BackgroundData) {
    let info = scen.get_info().expect("INFO is guaranteed");
    ui.label(format!("BG Index: {}",info.which_bg));
}
