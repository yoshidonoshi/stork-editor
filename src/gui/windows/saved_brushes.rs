use std::{cmp::Ordering, error::Error, fs::File, io::{BufReader, Write}, ops::Deref, sync::LazyLock};

use egui::{CursorIcon, TextEdit};
use egui_extras::{Column, TableBuilder};
use serde_json::json;

use crate::{data::backgrounddata::BackgroundData, engine::displayengine::DisplayEngine, gui::{windows::brushes::{BrushType, STORED_BRUSHES}}, utils::{log_write, LogLevel}};

use super::brushes::{Brush, StoredBrushes};

pub fn show_saved_brushes_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    if !de.display_settings.is_cur_layer_bg() {
        // Technically uneccesary, but the disabled appearance is good
        ui.disable();
    }
    let which_bg = de.display_settings.current_layer as u8;
    let layer: Option<&BackgroundData> = match which_bg {
        1 => de.bg_layer_1.as_ref(),
        2 => de.bg_layer_2.as_ref(),
        3 => de.bg_layer_3.as_ref(),
        _ => {
            ui.label(format!("Current layer is not a BG layer: '{:?}'",&de.display_settings.current_layer));
            return;
        }
    };
    let mut tileset_name = String::from("N/A");
    if let Some(bg_layer) = &layer {
        let imbz_noext = bg_layer.get_info().expect("saved_brushes layer has info").imbz_filename_noext.as_ref();
        if let Some(imbz_name) = imbz_noext {
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
            let mut create_brush_row = |i, brush_type, stamp: &Brush, saved_brushes: Option<&mut Vec<Brush>>| {
                // Tileset check
                if de.brush_settings.only_show_same_tileset {
                    if tileset_name != stamp.tileset {
                        return;
                    }
                }

                // Search check
                let stamp_name = stamp.name.trim().to_lowercase();
                let cur_search_string = de.brush_settings.cur_search_string.trim().to_lowercase();
                if !stamp_name.contains(&cur_search_string) {
                    return;
                }

                let tileset_match = tileset_name == stamp.tileset;
                body.row(20.0, |mut row| {
                    if let Some(selected_brush) = de.brush_settings.cur_selected_brush {
                        if tileset_match { // Don't let them select the wrong one
                            row.set_selected(selected_brush == (brush_type, i));
                        }
                    } // Otherwise nothing selected
                    
                    row.col(|ui| {
                        if !tileset_match {
                            ui.disable();
                        }
                        // TODO: remove interaction
                        let label_name = ui.label(&stamp.name);
                        if label_name.clicked() {
                            if tileset_match {
                                de.brush_settings.cur_selected_brush = Some((brush_type, i));
                                de.current_brush = stamp.clone();
                            }
                        }
                    });
                    row.col(|ui| {
                        if !tileset_match {
                            ui.disable();
                        }
                        // TODO: remove interaction
                        let tileset_label = ui.label(&stamp.tileset);
                        if tileset_label.clicked() {
                            if tileset_match {
                                de.brush_settings.cur_selected_brush = Some((brush_type, i));
                                de.current_brush = stamp.clone();
                            }
                        }
                    });

                    let response = row.response();

                    if response.clicked() {
                        if tileset_match {
                            de.brush_settings.cur_selected_brush = Some((brush_type, i));
                            de.current_brush = stamp.clone();
                        }
                    }

                    response.context_menu(|ui| {
                        ui.add_enabled_ui(saved_brushes.is_some(), |ui| {
                            let overwrite = ui.add_enabled_ui(
                                de.brush_settings.cur_selected_brush.is_some(),
                                |ui| ui.button("Overwrite")
                            ).inner;
                            let delete = ui.button("Delete");

                            if let Some(saved_brushes) = saved_brushes {
                                if overwrite.clicked() {
                                    let name = std::mem::take(&mut saved_brushes[i].name);
                                    saved_brushes[i] = de.current_brush.clone(); // this also clones the string name :/
                                    saved_brushes[i].name = name;
                                    save_brushes_to_file(saved_brushes);
                                }
                                if delete.clicked() {
                                    saved_brushes.remove(i);
                                    save_brushes_to_file(saved_brushes);
                                    // update selected brush index
                                    if let Some((_, ref mut sel_i)) = de.brush_settings.cur_selected_brush {
                                        match sel_i.deref().cmp(&i) {
                                            Ordering::Greater => *sel_i -= 1, // underflow is unreachable
                                            Ordering::Equal => de.brush_settings.cur_selected_brush = None,
                                            Ordering::Less => {}
                                        }
                                    }
                                }
                            }
                        });
                    });

                    response.on_hover_cursor(CursorIcon::PointingHand);
                });
            };

            for (i, stamp) in STORED_BRUSHES.brushes.iter().enumerate() {
                create_brush_row(i, BrushType::Stored, stamp, None)
            }
            for (i, stamp) in de.saved_brushes.clone().into_iter().enumerate() {
                create_brush_row(i, BrushType::Saved, &stamp, Some(&mut de.saved_brushes));
            }
        });
    ui.horizontal(|ui| {
        let store_enabled = !de.current_brush.tiles.is_empty();
        let button_store = ui.add_enabled(store_enabled, egui::Button::new("Store Current Brush"));
        if button_store.clicked() {
            let entered_brush_name = de.brush_settings.pos_brush_name.clone();
            if entered_brush_name.trim().is_empty() {
                log_write("Cannot save Brush with no name", LogLevel::WARN);
                return;
            }
            de.current_brush.name = entered_brush_name;
            de.current_brush.tileset = tileset_name.clone();
            de.current_brush.palette_offset = layer.expect("Layer should load in Stamps")._pal_offset;
            // Height, Width, Tiles already set in Brush window
            de.saved_brushes.push(de.current_brush.clone());
            de.brush_settings.pos_brush_name.clear();
            save_brushes_to_file(&de.saved_brushes);
        }
        if store_enabled {
            ui.text_edit_singleline(&mut de.brush_settings.pos_brush_name);
        } else {
            ui.add_enabled(false, TextEdit::singleline(&mut String::from("No tiles in current brush")));
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
            de.load_saved_brushes();
        }
    });
}

pub fn load_stored_brushes() {
    log_write("Loading Stored brushes...", LogLevel::DEBUG);
    LazyLock::force(&STORED_BRUSHES);
    log_write("Loaded stored brushes successfully", LogLevel::LOG);
}

const SAVED_BRUSHES_FILE: &str = "saved_brushes.json";

pub fn save_brushes_to_file(brushes: &Vec<Brush>) {
    log_write("Saving loaded Brushes to JSON...", LogLevel::LOG);
    let saved_brushes = json!({
        "brushes": brushes,
    });
    let pretty_string = serde_json::to_string_pretty(&saved_brushes).expect("Brushes should Stringify correctly");
    let mut output = File::create(SAVED_BRUSHES_FILE).expect("Can init the Brushes JSON file");
    if let Err(error) = write!(output,"{pretty_string}") {
        log_write(format!("Failed to write Brushes JSON: '{error}'"), LogLevel::ERROR);
    }
}

fn load_saved_brushes() -> Result<Vec<Brush>,Box<dyn Error>> {
    let file = match File::open(SAVED_BRUSHES_FILE) {
        Err(error) => {
            log_write(format!("Could not open {SAVED_BRUSHES_FILE}: '{error}'"), LogLevel::WARN);
            return Ok(Vec::new());
        }
        Ok(f) => f,
    };
    let reader = BufReader::new(file);
    let saved_brushes: StoredBrushes = serde_json::from_reader(reader)?;
    Ok(saved_brushes.brushes)
}

impl DisplayEngine {
    pub fn load_saved_brushes(&mut self) {
        log_write("Loading Saved brushes...", LogLevel::DEBUG);
        match load_saved_brushes() {
            Err(error) => {
                log_write(format!("Failed to load brushes from JSON: '{error}'"), LogLevel::ERROR);
            }
            Ok(brushes_load_attempt) => {
                self.saved_brushes = brushes_load_attempt;
                log_write("Loaded saved brushes successfully", LogLevel::LOG);
            }
        }
    }
}
