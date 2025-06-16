use egui::Color32;

use crate::{data::{scendata::{info::ScenInfoData, ScenSegment, ScenSegmentWrapper}, types::CurrentLayer}, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}, NON_MAIN_FOCUSED};

pub fn show_scen_segments_window(ui: &mut egui::Ui, de: &mut DisplayEngine, layer: &CurrentLayer) {
    puffin::profile_function!();
    let mut do_del: Option<usize> = Option::None;
    egui::ScrollArea::vertical()
    .auto_shrink(false)
    .min_scrolled_height(1.0)
    .show(ui, |ui| {
        let Some(bg) = de.loaded_map.get_background(*layer as u8) else {
            ui.label("Not on a loaded background layer");
            return;
        };
        for (i,seg) in &mut bg.scen_segments.iter_mut().enumerate() {
            let header = seg.header();
            let header = header.as_str();
            match header {
                "INFO" => {
                    ui.heading("INFO");
                    if let ScenSegmentWrapper::INFO(info) = seg {
                        let changed = show_info_segment(ui, info);
                        if changed {
                            log_write("Changed INFO", LogLevel::Debug);
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
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
            // Most SCEN segments are just too important to delete; all connected
            let is_deletable = header.eq("SCRL"); // So far this is the only easy one to handle
            let del_button = ui.add_enabled(is_deletable, egui::Button::new("Delete"));
            if del_button.clicked() {
                do_del = Some(i);
            }
            ui.separator();
        }
    });
    if let Some(to_del) = do_del {
        let bg = de.loaded_map.get_background(*layer as u8).expect("BG missing canceled earlier");
        let header = bg.scen_segments[to_del].header();
        log_write(format!("Deleting segment '{}' at index {}",header,to_del), LogLevel::Log);
        bg.scen_segments.remove(to_del);
        de.graphics_update_needed = true;
        de.unsaved_changes = true;
    }
}

fn show_info_segment(ui: &mut egui::Ui, info: &mut ScenInfoData) -> bool {
    let pre_change = info.clone();
    ui.horizontal(|ui| {
        ui.label(format!("0x{:04X}",info.layer_width));
        ui.label("Layer Width");
    });
    ui.horizontal(|ui| {
        ui.label(format!("0x{:04X}",info.layer_height));
        ui.label("Layer Height");
    });
    // Offset
    ui.horizontal(|ui| {
        let x_offset_drag = egui::DragValue::new(&mut info.x_offset_px)
            .speed(0x10)
            .hexadecimal(4, false, true)
            .range(i16::MIN..=i16::MAX);
        ui.add(x_offset_drag);
        ui.label("X Offset (px)").on_hover_ui(|ui| {
            ui.label("Higher numbers make the bg position move leftwards");
            ui.label("Each unit is 1 pixel moved, so it does not show up in the canvas yet");
        });
    });
    ui.horizontal(|ui| {
        let y_offset_drag = egui::DragValue::new(&mut info.y_offset_px)
            .speed(0x10)
            .hexadecimal(4, false, true)
            .range(i16::MIN..=i16::MAX);
        ui.add(y_offset_drag).on_hover_ui(|ui| {
            ui.label("Higher numbers make the bg position higher");
            ui.label("Each unit is 1 pixel moved, so it does not show up in the canvas yet");
        });
        ui.label("Y Offset (px)");
    });
    // Scroll
    ui.horizontal(|ui| {
        let scroll_drag = egui::DragValue::new(&mut info.x_scroll)
            .speed(0x100)
            .hexadecimal(8, false, true)
            .range(0..=0xffffff);
        ui.add(scroll_drag);
        ui.label("X Scroll");
        if ui.button("Match Ground").clicked() {
            info.x_scroll = 0x1000;
        }
    });
    ui.horizontal(|ui| {
        let scroll_drag = egui::DragValue::new(&mut info.y_scroll)
            .speed(0x100)
            .hexadecimal(8, false, true)
            .range(0..=0xffffff);
        ui.add(scroll_drag);
        ui.label("Y Scroll");
        if ui.button("Match Ground").clicked() {
            info.y_scroll = 0x1000;
        }
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.which_bg));
        ui.label("BG Index");
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.layer_order));
        ui.label("Layer Order");
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.char_base_block));
        ui.label("Char Base Block");
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}",info.screen_base_block));
        ui.label("Screen Base Block");
    });
    ui.horizontal(|ui| {
        let color_mode_drag = egui::DragValue::new(&mut info.color_mode)
            .speed(1)
            .range(0..=3);
        let cmres = ui.add_enabled(false,color_mode_drag);
        if cmres.has_focus() { // for the future
            *NON_MAIN_FOCUSED.lock().unwrap() = true;
        }
        ui.label("Color Mode");
    });
    if let Some(imbz_filename_noext) = &info.imbz_filename_noext {
        ui.horizontal(|ui| {
            ui.label(format!("'{imbz_filename_noext}'"));
            ui.label("Pixel Tile Source");
        });
    } else {
        ui.horizontal(|ui| {
            ui.label("'Local'");
            ui.label("Pixel Tile Source");
        });
    }
    // If it differs, return true and refresh
    pre_change != *info
}
