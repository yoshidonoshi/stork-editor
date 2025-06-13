use std::{collections::HashMap, fs};

use egui::Color32;
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use uuid::Uuid;

use crate::{data::course_file::{exit_type_name, CourseMapInfo, MapEntrance, MapExit}, engine::displayengine::DisplayEngine, utils::{self, log_write, nitrofs_abs, LogLevel}, NON_MAIN_FOCUSED};

pub struct CourseSettings {
    pub selected_map: Option<usize>,
    pub selected_entrance: Option<Uuid>,
    pub selected_exit: Option<Uuid>,
    pub add_window_open: bool,
    pub map_templates: HashMap<String,String>,
    pub add_map_selected: String
}
impl Default for CourseSettings {
    fn default() -> Self {
        Self {
            selected_map: None, selected_entrance: None,
            selected_exit: None, add_window_open: false,
            map_templates: utils::get_map_templates(),
            add_map_selected: "".to_string()
        }
    }
}

fn get_course_music_name(music: u8) -> String {
    let name = match music {
        0x0	=> "Flower Garden (dup?)",
        0x1	=> "Story Music Box",
        0x2	=> "Yoshi's Island DS",
        0x3	=> "Flower Field",
        0x4	=> "Yoshi's Island DS (dup?)",
        0x5	=> "Yoshi's Island DS (dup?)",
        0x6	=> "Training Course",
        0x7	=> "Score",
        0x8	=> "Minigame",
        0x9	=> "Flower Garden",
        0xA	=> "Underground",
        0xB	=> "Sea Coast",
        0xC	=> "Jungle",
        0xD	=> "Castle",
        0xE	=> "In The Clouds",
        0xF	=> "Wildlands",
        0x10 => "Bonus Challenge",
        0x11 => "Kamek's Theme",
        0x12 => "Mini-Boss",
        0x13 => "Boss Room",
        0x14 => "Big Boss",
        0x15 => "Flower Garden (dup?)",
        0x16 => "Bowser",
        0x17 => "Castle again?",
        0x18 => "Silence",
        0x19 => "Silence (Echoes)",
        _ => "Unknown"
    };
    String::from(name)
}

pub fn show_course_settings_window(ui: &mut egui::Ui, de: &mut DisplayEngine, project_open: bool) {
    puffin::profile_function!();
    StripBuilder::new(ui)
        .size(Size::exact(100.0))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                draw_map_section(ui, de, project_open);
            });
            strip.cell(|ui| {
                draw_settings_section(ui, de);
            });
        });
}

fn draw_map_section(ui: &mut egui::Ui, de: &mut DisplayEngine, project_open: bool) {
    ui.horizontal(|ui| {
        if !project_open {
            ui.disable(); // Project is closed
        }
        let new_button = ui.button("New");
        if new_button.clicked() {
            de.course_settings.add_window_open = true;
        }
        if de.course_settings.selected_map.unwrap_or(0xffff) == de.map_index.unwrap_or(0xDEADBEEF) {
            // Don't delete the active map
            ui.disable();
        }
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
        let delete_button = ui.button("Delete");
        if delete_button.hovered() {
            egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("delete_map_warning"), |ui| {
                ui.label("WARNING: THIS CANNOT BE UNDONE");
                ui.label("Hold shift and click to confirm deletion");
            });
        }
        if delete_button.clicked() {
            if !ui.input(|i| i.modifiers.shift) {
                log_write("Shift must be held down to delete maps", LogLevel::Log);
                return;
            }
            let Some(selected_map_index) = de.course_settings.selected_map else {
                log_write("No map selected", LogLevel::Debug);
                return;
            };
            if selected_map_index >= de.loaded_course.level_map_data.len() {
                log_write("Selected map overflow when deleting, resetting", LogLevel::Error);
                de.course_settings.selected_map = None;
                return;
            }
            log_write("Deleting selected Map", LogLevel::Log);
            let file_name = &de.loaded_course.level_map_data[selected_map_index].map_filename_noext;
            let file_to_delete = nitrofs_abs(de.export_folder.to_path_buf(), &format!("{}.mpdz",file_name));
            let _did_delete = de.loaded_course.delete_map_info_by_index(selected_map_index);
            log_write(format!("Deleting file '{}'...",&file_to_delete.display()), LogLevel::Debug);
            let del_res = fs::remove_file(&file_to_delete);
            match del_res {
                Ok(_) => log_write(format!("Deleted file '{}' successfully",&file_to_delete.display()), LogLevel::Log),
                Err(e) => {
                    log_write(format!("Failed to delete file '{}': '{}'",&file_to_delete.display(),e), LogLevel::Error);
                    return;
                }
            }
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
            de.course_settings.selected_map = None;
        }
    });
    ui.add_space(5.0);
    let _table = TableBuilder::new(ui)
        .striped(true)
        .column(Column::exact(100.0))
        .sense(egui::Sense::click())
        .body(|mut body| {
            for map in &de.loaded_course.level_map_data {
                body.row(20.0, |mut row| {
                    let row_index = row.index();
                    row.set_selected(de.course_settings.selected_map.unwrap_or(0xffff) == row_index);
                    row.col(|ui| {
                        let label = ui.label(&map.label);
                        if label.clicked() {
                            de.course_settings.selected_map = Some(row_index);
                        }
                    });
                    if row.response().clicked() {
                        de.course_settings.selected_map = Some(row_index);
                    }
                });
            }
        });
}

fn draw_settings_section(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    let Some(selected_map_index) = de.course_settings.selected_map else {
        ui.label("No Map selected");
        return;
    };
    let Some(stored_map_data) = de.loaded_course.level_map_data.get(selected_map_index).cloned() else {
        log_write("Selected map index out of bounds, clearing", LogLevel::Error);
        de.course_settings.selected_map = Option::None;
        return;
    };
    // MUSIC //
    let selected_map_data = &mut de.loaded_course.level_map_data[selected_map_index];
    let old_map_music_val = selected_map_data.map_music;
    ui.heading("Music");
    egui::ComboBox::from_label("")
        .selected_text(format!("0x{:02X} - {}",selected_map_data.map_music,get_course_music_name(selected_map_data.map_music)))
        .show_ui(ui, |ui| {
            for x in 0..=23 { // 23 is the highest value found in all CRSBs via script
                ui.selectable_value(&mut selected_map_data.map_music, x, get_course_music_name(x));
            }
        });
    if old_map_music_val != selected_map_data.map_music {
        log_write(format!("Changed Map music index to '{}'",&selected_map_data.map_music), LogLevel::Log);
        de.unsaved_changes = true;
    }
    ui.separator();
    // ENTRANCES //
    ui.heading("Entrances");
    ui.horizontal(|ui| {
        let add = ui.add(egui::Button::new("New"));
        if add.clicked() {
            let selected_map_data = &mut de.loaded_course.level_map_data[selected_map_index];
            let new_uuid = selected_map_data.add_entrance();
            de.course_settings.selected_entrance = Some(new_uuid);
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
            // This won't mess with anything
            log_write("New Entrance created", LogLevel::Log);
        }
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
        // Don't let it delete the last one, should always be at least 1
        let entrance_count = de.loaded_course.level_map_data[selected_map_index].map_entrances.len();
        let del = ui.add_enabled(de.course_settings.selected_entrance.is_some() && entrance_count > 1,
            egui::Button::new("Delete"));
        if del.clicked() {
            log_write("Deleting Entrance", LogLevel::Debug);
            let deld = de.loaded_course.level_map_data[selected_map_index]
                .delete_entrance(de.course_settings.selected_entrance.expect("selected entrance checked earlier"));
            // Deselect regardless of result
            de.course_settings.selected_entrance = Option::None;
            if !deld {
                return;
            }
            de.loaded_course.fix_exits();
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
        }
    });
    ui.horizontal(|ui| {
        let selected_map_data = &mut de.loaded_course.level_map_data[selected_map_index];
        let _table_entrances = TableBuilder::new(ui)
        .id_salt("entrances")
        .striped(true)
        .column(Column::exact(100.0))
        .sense(egui::Sense::click())
        .body(|mut body| {
            for entrance in &selected_map_data.map_entrances {
                body.row(20.0, |mut row| {
                    row.set_selected(de.course_settings.selected_entrance.unwrap_or(Uuid::nil()) == entrance.uuid);
                    row.col(|ui| {
                        let label = ui.label(&entrance.label);
                        if label.clicked() {
                            de.course_settings.selected_entrance = Some(entrance.uuid);
                        }
                    });
                    if row.response().clicked() {
                        de.course_settings.selected_entrance = Some(entrance.uuid);
                    }
                });
            }
        });
        ui.vertical(|ui| {
            let Some(selected_entrance_uuid) = de.course_settings.selected_entrance else {
                return;
            };
            if selected_entrance_uuid == Uuid::nil() {
                return;
            }
            let Some(selected_entrance) = selected_map_data.get_entrance_mut(&selected_entrance_uuid) else { return };
            // Begin selected Entrance settings
            ui.horizontal(|ui| {
                let drag_value_x = egui::DragValue::new(&mut selected_entrance.entrance_x)
                    .hexadecimal(4, false, true)
                    .range(0..=0xffff);
                let dvx = ui.add(drag_value_x);
                if dvx.has_focus() {
                    *NON_MAIN_FOCUSED.lock().unwrap() = true;
                }
                let drag_value_y = egui::DragValue::new(&mut selected_entrance.entrance_y)
                    .hexadecimal(4, false, true)
                    .range(0..=0xffff);
                let dvy = ui.add(drag_value_y);
                if dvy.has_focus() {
                    *NON_MAIN_FOCUSED.lock().unwrap() = true;
                }
            });
            show_selected_entrance_settings(ui, selected_entrance);
        });
    });
    ui.separator();
    // EXITS //
    ui.heading("Exits");
    ui.horizontal(|ui| {
        let add = ui.add(egui::Button::new("New"));
        if add.clicked() {
            let selected_map_data = &mut de.loaded_course.level_map_data[selected_map_index];
            let new_uuid = selected_map_data.add_exit();
            // New exits have error ids
            de.loaded_course.fix_exits();
            de.course_settings.selected_exit = Some(new_uuid);
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
            log_write("New exit created", LogLevel::Log);
        }
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
        // Don't let it delete the last one, should always be at least 1
        let exit_count = de.loaded_course.level_map_data[selected_map_index].map_exits.len();
        let del = ui.add_enabled(de.course_settings.selected_exit.is_some() && exit_count > 1,
            egui::Button::new("Delete"));
        if del.clicked() {
            log_write("Deleting Exit", LogLevel::Debug);
            let deld = de.loaded_course.level_map_data[selected_map_index]
                .delete_exit(de.course_settings.selected_exit.expect("selected exit checked earlier"));
            // Deselect regardless of result
            de.course_settings.selected_exit = Option::None;
            if !deld {
                return;
            }
            // Nothing links to an exit, no need to check anything
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
        }
    });
    ui.horizontal(|ui| {
        let _table_exits = TableBuilder::new(ui)
        .id_salt("exits")
        .striped(true)
        .column(Column::exact(100.0))
        .sense(egui::Sense::click())
        .body(|mut body| {
            let selected_map_data = &mut de.loaded_course.level_map_data[selected_map_index];
            for exit in &selected_map_data.map_exits {
                body.row(20.0, |mut row| {
                    row.set_selected(de.course_settings.selected_exit.unwrap_or(Uuid::nil()) == exit.uuid);
                    row.col(|ui| {
                        let label = ui.label(&exit.label);
                        if label.clicked() {
                            de.course_settings.selected_exit = Some(exit.uuid);
                        }
                    });
                    if row.response().clicked() {
                        de.course_settings.selected_exit = Some(exit.uuid);
                    }
                });
            }
        });
        ui.vertical(|ui| {
            let ro_map_data = de.loaded_course.level_map_data.clone();
            let Some(selected_exit) = de.get_selected_exit_mut() else {
                ui.label("No Exit selected");
                return;
            };
            // Here is where the Exit settings are once selected
            show_exit_pos(ui, selected_exit);
            show_exit_type(ui, selected_exit);
            show_exit_target_map(ui, selected_exit,&ro_map_data);
            show_exit_target_entrance(ui, selected_exit, &ro_map_data);
        });
    });
    ui.separator();
    if de.loaded_course.level_map_data[selected_map_index] != stored_map_data {
        de.unsaved_changes = true;
    }
}

fn show_selected_entrance_settings(ui: &mut egui::Ui, selected_entrance: &mut MapEntrance) {
    let which_screen = selected_entrance.entrance_flags >> 14;
    let enter_map_anim = selected_entrance.entrance_flags % 0x1000;
    ui.label(format!("Raw Flags: {:X}",selected_entrance.entrance_flags));
    ui.label(format!("Which Screen: {:X}",which_screen));
    ui.label(format!("Entrance Animation?: {:X}",enter_map_anim));
}

fn show_exit_pos(ui: &mut egui::Ui, selected_exit: &mut MapExit) {
    ui.horizontal(|ui| {
        let drag_value_x = egui::DragValue::new(&mut selected_exit.exit_x)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        let dvx = ui.add(drag_value_x);
        if dvx.has_focus() {
            *NON_MAIN_FOCUSED.lock().unwrap() = true;
        }
        let drag_value_y = egui::DragValue::new(&mut selected_exit.exit_y)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        let dvy = ui.add(drag_value_y);
        if dvy.has_focus() {
            *NON_MAIN_FOCUSED.lock().unwrap() = true;
        }
    });
}

fn show_exit_type(ui: &mut egui::Ui, selected_exit: &mut MapExit) {
    let _exit_type_dropdown = egui::ComboBox::from_label("Type")
        .selected_text(exit_type_name(selected_exit.exit_type))
        .show_ui(ui, |ui| {
            for x in 0..=0xE { // 0xE is highest found in previous Stork, confirmed by script
                ui.selectable_value(&mut selected_exit.exit_type,
                    x, exit_type_name(x));
            }
        });
}

fn show_exit_target_map(ui: &mut egui::Ui, selected_exit: &mut MapExit, maps: &[CourseMapInfo]) {
    let Some(course) = maps.iter().find(|x| x.uuid == selected_exit.target_map) else {
        log_write("Somehow, course was none", LogLevel::Error);
        return;
    };
    let selected_exit_stored = selected_exit.clone();
    let _exit_target_map_dropdown = egui::ComboBox::from_label("Target Map")
        .selected_text(&course.label)
        .show_ui(ui, |ui| {
            for map in maps {
                ui.selectable_value(&mut selected_exit.target_map, map.uuid, &map.label);
            }
        });
    if selected_exit_stored != *selected_exit {
        // The new entrance will be invalid! Reset it to the first one
        log_write("Selected exit target map changed", LogLevel::Debug);
        let Some(course_new) = maps.iter().find(|x| x.uuid == selected_exit.target_map) else {
            log_write("Failed to find course for selected exit target map", LogLevel::Fatal);
            unreachable!()
        };
        let Some(first_map_entrance) = course_new.map_entrances.first() else {
            log_write("New exit target map has no entrances",LogLevel::Fatal);
            unreachable!()
        };
        selected_exit.target_map_entrance = first_map_entrance.uuid;
    }
}

fn show_exit_target_entrance(ui: &mut egui::Ui, selected_exit: &mut MapExit, maps: &[CourseMapInfo]) {
    let Some(course) = maps.iter().find(|x| x.uuid == selected_exit.target_map) else {
        log_write("Somehow, course was none", LogLevel::Error);
        return;
    };
    let Some(cur_entrance) = course.get_entrance(&selected_exit.target_map_entrance) else {
        return;
    };
    let _exit_target_map_dropdown = egui::ComboBox::from_label("Target Entrance")
        .selected_text(&cur_entrance.label)
        .show_ui(ui, |ui| {
            for enter in &course.map_entrances {
                ui.selectable_value(&mut selected_exit.target_map_entrance, enter.uuid, &enter.label);
            }
        });
}
