use crate::{data::{scendata::{info::ScenInfoData, ScenSegment, ScenSegmentWrapper}, types::CurrentLayer}, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

pub fn show_scen_segments_window(ui: &mut egui::Ui, de: &mut DisplayEngine, layer: &CurrentLayer) {
    egui::ScrollArea::vertical()
    .auto_shrink(false)
    .min_scrolled_height(1.0)
    .show(ui, |ui| {
        let bg = de.loaded_map.get_background(*layer as u8);
        if bg.is_none() {
            ui.label("Not on a loaded background layer");
            return;
        }
        let bg = bg.unwrap();
        for seg in &mut bg.scen_segments {
            match seg.header().as_str() {
                "INFO" => {
                    ui.heading("INFO");
                    if let ScenSegmentWrapper::INFO(info) = seg {
                        let changed = show_info_segment(ui, info);
                        if changed {
                            log_write(format!("Changed INFO"), LogLevel::DEBUG);
                            de.unsaved_changes = true;
                            de.graphics_update_needed = true;
                        }
                    } else {
                        ui.label("ERROR: Could not retrieve INFO");
                    }
                }
                "COLZ" => {
                    ui.heading("COLZ");
                    if let ScenSegmentWrapper::COLZ(colz) = seg {
                        let coz_len = colz.col_tiles.len();
                        ui.label(format!("Tile count: 0x{:X} ({})",coz_len,coz_len));
                    } else {
                        ui.label("ERROR: Could not retrieve COLZ");
                    }
                }
                "PLTB" => {
                    ui.heading("PLTB");
                    if let ScenSegmentWrapper::PLTB(pltb) = seg {
                        let pal_count = pltb.palettes.len();
                        ui.label(format!("Palette count: 0x{:X} ({})",pal_count,pal_count));
                    } else {
                        ui.label("ERROR: Could not retrieve PLTB");
                    }
                }
                "MPBZ" => {
                    ui.heading("MPBZ");
                    if let ScenSegmentWrapper::MPBZ(mpbz) = seg {
                        let map_tile_count = mpbz.tiles.len();
                        ui.label(format!("Map Tile count: 0x{:X} ({})",map_tile_count,map_tile_count));
                        ui.label(format!("Bottom Trim: 0x{:X} ({})",mpbz.bottom_trim,mpbz.bottom_trim));
                        ui.label(format!("Tile Offset: 0x{:X} ({})",mpbz.tile_offset,mpbz.tile_offset));
                    } else {
                        ui.label("ERROR: Could not retrieve MPBZ");
                    }
                }
                "SCRL" => {
                    ui.heading("SCRL");
                    if let ScenSegmentWrapper::SCRL(scrl) = seg {
                        ui.label(format!("Left Velocity: {}",scrl.left_velocity));
                        ui.label(format!("Upwards Velocity: {}",scrl.up_velocity));
                    } else {
                        ui.label("ERROR: Could not retrieve SCRL");
                    }
                }
                "ANMZ" => {
                    ui.heading("ANMZ");
                    if let ScenSegmentWrapper::ANMZ(anmz) = seg {
                        let anmz_len = anmz.pixeltiles.len();
                        ui.label(format!("Tile count: 0x{:X} ({})",anmz_len,anmz_len));
                        ui.label(format!("Frame count: {}",anmz.frame_count));
                        ui.label(format!("Frame Holds: {:?}",anmz.frame_holds));
                    } else {
                        ui.label("ERROR: Could not retrieve ANMZ");
                    }
                }
                "IMGB" => {
                    ui.heading("IMGB");
                    if let ScenSegmentWrapper::IMGB(imgb) = seg {
                        let tile_count = imgb.pixel_tiles.len();
                        ui.label(format!("PixelTile count: 0x{:X} ({})",tile_count,tile_count));
                    } else {
                        ui.label("ERROR: Could not retrieve IMGB");
                    }
                }
                "IMBZ" => {
                    ui.heading("IMBZ");
                    if let ScenSegmentWrapper::IMBZ(imbz) = seg {
                        let tile_count = imbz.pixel_tiles.len();
                        ui.label(format!("PixelTile count: 0x{:X} ({})",tile_count,tile_count));
                    } else {
                        ui.label("ERROR: Could not retrieve IMBZ");
                    }
                }
                "PLAN" => {
                    ui.heading("PLAN");
                    if let ScenSegmentWrapper::PLAN(plan) = seg {
                        ui.label(format!("Raw Size in Bytes: 0x{:X}",plan._raw.len()));
                    } else {
                        ui.label("ERROR: Could not retrieve PLAN");
                    }
                }
                "RAST" => {
                    ui.heading("RAST");
                    if let ScenSegmentWrapper::RAST(rast) = seg {
                        ui.label(format!("Raw Size in Bytes: 0x{:X}",rast._raw.len()));
                    } else {
                        ui.label("ERROR: Could not retrieve RAST");
                    }
                }
                _ => {
                    ui.label(format!("Unhandled segment: '{}'",&seg.header()));
                }
            }
            ui.separator();
        }
    });
}

fn show_info_segment(ui: &mut egui::Ui, info: &mut ScenInfoData) -> bool {
    let pre_change = info.clone();
    ui.horizontal(|ui| {
        ui.label(format!("0x{:04X}",info.layer_width));
        ui.label(format!("Layer Width"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("0x{:04X}",info.layer_height));
        ui.label(format!("Layer Height"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("0x{:08X}",info.height_offset));
        ui.label(format!("Height Offset (Fine)"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("0x{:08X}",info.x_scroll));
        ui.label(format!("X Scroll"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("0x{:08X}",info.y_scroll));
        ui.label(format!("Y Scroll"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.which_bg));
        ui.label(format!("BG Index"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.layer_order));
        ui.label(format!("Layer Order"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.char_base_block));
        ui.label(format!("Char Base Block"));
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.screen_base_block));
        ui.label(format!("Screen Base Block"));
    });
    ui.horizontal(|ui| {
        let color_mode_drag = egui::DragValue::new(&mut info.color_mode)
            .speed(1)
            .range(0..=3);
        ui.add_enabled(false,color_mode_drag);
        ui.label(format!("Color Mode"));
    });
    if info.imbz_filename_noext.is_none() {
        ui.horizontal(|ui| {
            ui.label(format!("'Local'"));
            ui.label(format!("Pixel Tile Source"));
        });
    } else {
        ui.horizontal(|ui| {
            ui.label(format!("'{}'",info.imbz_filename_noext.clone().unwrap()));
            ui.label(format!("Pixel Tile Source"));
        });
    }
    // If it differs, return true and refresh
    pre_change != *info
}
