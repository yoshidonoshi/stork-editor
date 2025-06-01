use std::{error::Error, fs::File, io::{BufReader, Write}, sync::LazyLock};

use egui::TextEdit;
use egui_extras::{Column, TableBuilder};
use serde_json::json;

use crate::{data::backgrounddata::BackgroundData, engine::displayengine::DisplayEngine, gui::{gui::Gui, windows::brushes::STORED_BRUSHES}, utils::{log_write, LogLevel}};

use super::brushes::{Brush, StoredBrushes};

pub fn show_saved_brushes_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
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
    let mut tileset_name = String::from("N/A");
    if let Some(bg_layer) = &layer {
        let imbz_noext = &bg_layer.get_info().expect("saved_brushes layer has info").imbz_filename_noext;
        if let Some(imbz_name) = &imbz_noext {
            tileset_name = imbz_name.clone();
        } else {
            ui.label("Non-IMBZ layers not yet supported");
            ui.disable();
        }
    } else {
        ui.label(format!("Current layer is not loaded: '{}'",which_bg));
        return;
    }
    ui.label(format!("Current tileset file: '{}'", tileset_name));
    let checkbox = ui.checkbox(&mut de.brush_settings.only_show_same_tileset, "Only show same tileset");
    if checkbox.hovered() {
        egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("same_tileset_checked"), |ui| {
            ui.label("Some tilesets have similar Brush names, but aren't compatible");
        });
    }
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
            for (i, stamp) in get_all_brushes(&de.saved_brushes).enumerate() {
                if de.brush_settings.only_show_same_tileset {
                    if tileset_name != stamp.tileset {
                        continue;
                    }
                }
                let tileset_match = tileset_name == stamp.tileset;
                body.row(20.0, |mut row| {
                    if let Some(sel_stamp) = &de.brush_settings.cur_selected_brush {
                        if tileset_match { // Don't let them select the wrong one
                            row.set_selected(*sel_stamp == row.index());
                        }
                    } // Otherwise nothing selected

                    let stamp_name = stamp.name.clone().to_lowercase();
                    let cur_search_string = &de.brush_settings.cur_search_string.clone();
                    if !stamp_name.contains(cur_search_string.trim()) {
                        return;
                    }
                    
                    row.col(|ui| {
                        if !tileset_match {
                            ui.disable();
                        }
                        let label_name = ui.label(&stamp.name);
                        if label_name.clicked() {
                            if tileset_match {
                                de.brush_settings.cur_selected_brush = Some(i);
                                de.current_brush = stamp.clone();
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
                                de.brush_settings.cur_selected_brush = Some(i);
                                de.current_brush = stamp.clone();
                            }
                        }
                    });

                    if row.response().clicked() {
                        if tileset_match {
                            de.brush_settings.cur_selected_brush = Some(i);
                            de.current_brush = stamp.clone();
                        }
                    }
                });
            }
        });
    ui.horizontal(|ui| {
        let mut store_enabled = true;
        let mut reason_disabled: String = String::from("ERROR");
        if de.current_brush.tiles.is_empty() {
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

        }
    });
}

#[inline]
fn get_all_brushes(saved_brushes: &Vec<Brush>) -> Box<dyn Iterator<Item = &Brush> + '_> {
    Box::new(STORED_BRUSHES.brushes.iter().chain(saved_brushes))
}

pub fn load_stored_brushes() {
    log_write("Loading Stored brushes...", LogLevel::DEBUG);
    LazyLock::force(&STORED_BRUSHES);
    log_write("Loaded stored brushes successfully", LogLevel::LOG);
}

const LOCAL_BRUSHES_FILE: &str = "saved_brushes.json";

pub fn save_brushes_to_file(brushes: &Vec<Brush>) {
    log_write("Saving loaded Brushes to JSON...", LogLevel::LOG);
    let saved_brushes = json!({
        "brushes": brushes,
    });
    let pretty_string = serde_json::to_string_pretty(&saved_brushes).expect("Brushes should Stringify correctly");
    let mut output = File::create(LOCAL_BRUSHES_FILE).expect("Can init the Brushes JSON file");
    if let Err(error) = write!(output,"{pretty_string}") {
        log_write(format!("Failed to write Brushes JSON: '{error}'"), LogLevel::ERROR);
    }
}

fn load_saved_brushes() -> Result<Vec<Brush>,Box<dyn Error>> {
    let file = match File::open(LOCAL_BRUSHES_FILE) {
        Err(error) => {
            log_write(format!("Could not open {LOCAL_BRUSHES_FILE}: '{error}'"), LogLevel::WARN);
            return Ok(Vec::new());
        }
        Ok(f) => f,
    };
    let reader = BufReader::new(file);
    let saved_brushes: StoredBrushes = serde_json::from_reader(reader)?;
    Ok(saved_brushes.brushes)
}

impl Gui {
    pub fn load_saved_brushes(&mut self) {
        log_write("Loading Saved brushes...", LogLevel::DEBUG);
        match load_saved_brushes() {
            Err(error) => {
                log_write(format!("Failed to load brushes from JSON: '{error}'"), LogLevel::ERROR);
            }
            Ok(brushes_load_attempt) => {
                self.display_engine.saved_brushes = brushes_load_attempt;
                log_write("Loaded saved brushes successfully", LogLevel::LOG);
            }
        }
    }
}
