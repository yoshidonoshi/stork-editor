use std::{error::Error, fs::File, io::{BufReader, Write}};

use egui::TextEdit;
use egui_extras::{Column, TableBuilder};
use serde_json::{json, Value};

use crate::{data::backgrounddata::BackgroundData, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

use super::brushes::Brush;

pub fn show_saved_brushes_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    if !de.display_settings.is_cur_layer_bg() {
        // Technically uneccesary, but the disabled appearance is good
        ui.disable();
    }
    let which_bg = de.display_settings.current_layer as u8;
    let layer: &Option<BackgroundData> = match which_bg {
        1 => &de.bg_layer_1,
        2 => &de.bg_layer_2,
        3 => &de.bg_layer_3,
        _ => {
            ui.label(format!("Current layer is not a BG layer: '{:?}'",&de.display_settings.current_layer));
            return;
        }
    };
    if layer.is_none() {
        ui.label(format!("Current layer is not loaded: '{}'",which_bg));
        return;
    }
    let mut tileset_name = String::from("N/A");
    if let Some(bg_layer) = &layer {
        if let Some(imbz_name) = &bg_layer.info_ro.imbz_filename_noext {
            tileset_name = imbz_name.clone();
        } else {
            ui.label("Non-IMBZ layers not yet supported");
            ui.disable();
        }
    } else {
        // Logically impossible
        log_write(format!("Failed to load BG layer '{}' in show_stamps_window",which_bg), LogLevel::FATAL);
        return;
    }
    ui.label(format!("Current tileset file: '{}'", tileset_name));
    ui.checkbox(&mut de.brush_settings.only_show_same_tileset, "Only show same tileset");
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add_enabled(true, egui::TextEdit::singleline(&mut de.brush_settings.cur_search_string));
    });
    let _table = TableBuilder::new(ui)
        .striped(true)
        .column(Column::remainder())
        .column(Column::exact(80.0))
        .sense(egui::Sense::click())
        .drag_to_scroll(false)
        .max_scroll_height(400.0)
        .body(|mut body| {
            for stamp in &de.saved_brushes {
                if de.brush_settings.only_show_same_tileset {
                    if tileset_name != stamp.tileset {
                        continue;
                    }
                }
                let tileset_match = tileset_name == stamp.tileset;
                body.row(20.0, |mut row| {
                    let row_index = row.index(); // Copies
                    if let Some(sel_stamp) = &de.brush_settings.cur_selected_brush {
                        if tileset_match { // Don't let them select the wrong one
                            row.set_selected(*sel_stamp == row.index());
                        }
                    } // Otherwise nothing selected

                    let stamp_name = stamp.name.clone().to_lowercase();
                    let cur_search_string = &de.brush_settings.cur_search_string.clone();
                    if !stamp_name.contains(&cur_search_string.trim()) {
                        return;
                    }
                    
                    row.col(|ui| {
                        if !tileset_match {
                            ui.disable();
                        }
                        let label_name = ui.label(&stamp.name);
                        if label_name.clicked() {
                            if tileset_match {
                                de.brush_settings.cur_selected_brush = Some(row_index);
                                let got_brush = get_selected_brush_data(&de.saved_brushes, row_index);
                                if got_brush.is_ok() {
                                    de.current_brush = got_brush.unwrap();
                                }
                            }
                        }
                    });
                    row.col(|ui| {
                        if !tileset_match {
                            ui.disable();
                        }
                        let tileset_label = ui.label(&stamp.tileset);
                        if tileset_label.clicked() {
                            if tileset_match {
                                de.brush_settings.cur_selected_brush = Some(row_index);
                                let got_brush = get_selected_brush_data(&de.saved_brushes, row_index);
                                if got_brush.is_ok() {
                                    de.current_brush = got_brush.unwrap();
                                }
                            }
                        }
                    });

                    if row.response().clicked() {
                        if tileset_match {
                            de.brush_settings.cur_selected_brush = Some(row_index);
                            let got_brush = get_selected_brush_data(&de.saved_brushes, row_index);
                            if got_brush.is_ok() {
                                de.current_brush = got_brush.unwrap();
                            }
                        }
                    }
                });
            }
        });
    ui.horizontal(|ui| {
        let mut store_enabled = true;
        let mut reason_disabled: String = String::from("ERROR");
        if de.current_brush.tiles.len() == 0 {
            store_enabled = false;
            reason_disabled = String::from("No tiles in current brush");
        }
        let button_store = ui.add_enabled(store_enabled, egui::Button::new("Store Current Brush"));
        if button_store.clicked() {
            de.current_brush.name = de.brush_settings.pos_brush_name.clone();
            de.current_brush.tileset = tileset_name.clone();
            // This is so janky... Damnit Rust
            de.current_brush.palette_offset = layer.clone().expect("Layer should load in Stamps")._pal_offset;
            // Height, Width, Tiles already set in Brush window
            de.saved_brushes.push(de.current_brush.clone());
            de.brush_settings.pos_brush_name.clear();
        }
        if store_enabled {
            ui.text_edit_singleline(&mut de.brush_settings.pos_brush_name);
        } else {
            ui.add_enabled(false, TextEdit::singleline(&mut reason_disabled));
        }
    });
    ui.horizontal(|ui| {
        let brush_export_button = ui.button("Export Brushes JSON");
        if brush_export_button.clicked() {
            save_brushes_to_file(&de.saved_brushes);
        }
        ui.disable();
        let brush_load_button = ui.button("Load Brushes JSON");
        if brush_load_button.clicked() {
            let brushes_load_attempt = load_brushes_from_file();
            if brushes_load_attempt.is_err() {
                log_write(format!("Failed to load brushes from JSON: '{}'",brushes_load_attempt.unwrap_err()), LogLevel::ERROR);
            } else {
                de.saved_brushes = brushes_load_attempt.unwrap();
            }
        }
    });
}

fn get_selected_brush_data(saved_brushes: &Vec<Brush>, sel_brush_index: usize) -> Result<Brush,()> {
    if sel_brush_index >= saved_brushes.len() {
        log_write("Selected Brush index out of bounds", LogLevel::ERROR);
        return Err(());
    }
    let brush_to_load = saved_brushes[sel_brush_index].clone();
    Ok(brush_to_load)
}

pub fn save_brushes_to_file(brushes: &Vec<Brush>) {
    log_write("Saving loaded Brushes to JSON...", LogLevel::LOG);
    let mut out_json = json!({
        "brushes": []
    });
    for brush in brushes {
        let j_string = serde_json::to_value(brush);
        if j_string.is_err() {
            log_write(format!("Failed to convert Brush '{}' to JSON: '{}'",brush.name,j_string.unwrap_err()), LogLevel::ERROR);
            return;
        }
        out_json["brushes"].as_array_mut().expect("Get output JSON as mutable array").push(j_string.unwrap());
    }
    let pretty_string = serde_json::to_string_pretty(&out_json).expect("Brushes should Stringify correctly");
    let mut output = File::create("stored_brushes.json").expect("Can init the Brushes JSON file");
    let json_write = write!(output,"{}",pretty_string);
    if json_write.is_err() {
        log_write(format!("Failed to write Brushes JSON: '{}'",json_write.unwrap_err()), LogLevel::ERROR);
    }
}

pub fn load_brushes_from_file() -> Result<Vec<Brush>,Box<dyn Error>> {
    let file: Result<File, std::io::Error> = File::open("stored_brushes.json");
    if file.is_err() {
        log_write(format!("Could not open stored_brushes.json: '{}'",file.unwrap_err()), LogLevel::WARN);
        return Ok(Vec::new());
    }
    let file = file.unwrap();
    let reader = BufReader::new(file);
    let j: Value = serde_json::from_reader(reader)?;
    let brush_json_array = j["brushes"].as_array().expect("Brushes JSON array created");
    let mut out_array: Vec<Brush> = Vec::new();
    for brush_value in brush_json_array {
        let b: Brush = serde_json::from_value(brush_value.clone())?;
        out_array.push(b);
    }
    Ok(out_array)
}
