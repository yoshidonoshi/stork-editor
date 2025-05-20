use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use uuid::Uuid;

use crate::{data::course_file::{exit_type_name, CourseMapInfo, MapEntrance, MapExit}, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

#[derive(Default)]
pub struct CourseSettings {
    pub selected_map: Option<usize>,
    pub selected_entrance: Option<Uuid>,
    pub selected_exit: Option<Uuid>
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

pub fn show_course_settings_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    StripBuilder::new(ui)
        .size(Size::exact(100.0))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                draw_map_section(ui, de);
            });
            strip.cell(|ui| {
                draw_settings_section(ui, de);
            });
        });
}

fn draw_map_section(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("New"));
        ui.add_enabled(false, egui::Button::new("Delete"));
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
                        let label = ui.label(map.label.to_string());
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
    if de.course_settings.selected_map.is_none() {
        ui.label("No Map selected");
        return;
    }
    let selected_map_index = de.course_settings.selected_map.unwrap();
    if selected_map_index >= de.loaded_course.level_map_data.len() {
        log_write("Selected map index out of bounds, clearing", LogLevel::ERROR);
        de.course_settings.selected_map = Option::None;
        return;
    }
    let stored_map_data = de.loaded_course.level_map_data[selected_map_index].clone();
    // MUSIC //
    { // This allows borrowing map data properly
        let selected_map_data = &mut de.loaded_course.level_map_data[selected_map_index];
        let old_map_music_val = selected_map_data.map_music.clone();
        ui.heading("Music");
        egui::ComboBox::from_label("")
            .selected_text(format!("0x{:02X} - {}",selected_map_data.map_music,get_course_music_name(selected_map_data.map_music)))
            .show_ui(ui, |ui| {
                for x in 0..=23 { // 23 is the highest value found in all CRSBs via script
                    ui.selectable_value(&mut selected_map_data.map_music, x, get_course_music_name(x));
                }
            });
        if old_map_music_val != selected_map_data.map_music {
            log_write(format!("Changed Map music index to '{}'",&selected_map_data.map_music), LogLevel::LOG);
            de.unsaved_changes = true;
        }
    } // Return borrowed mutable map data
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
        }
        ui.add_enabled(false, egui::Button::new("Delete"));
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
                        let label = ui.label(entrance.label.to_string());
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
            if de.course_settings.selected_entrance.is_none() {
                return;
            }
            let selected_entrance_uuid = de.course_settings.selected_entrance.expect("selected entrance passed nonecheck already");
            if selected_entrance_uuid == Uuid::nil() {
                return;
            }
            let selected_entrance = selected_map_data.get_entrance_mut(&selected_entrance_uuid);
            if selected_entrance.is_none() {
                return;
            }
            let selected_entrance = selected_entrance.unwrap();
            // Begin selected Entrance settings
            ui.horizontal(|ui| {
                let drag_value_x = egui::DragValue::new(&mut selected_entrance.entrance_x)
                    .hexadecimal(4, false, true)
                    .range(0..=0xffff);
                ui.add(drag_value_x);
                let drag_value_y = egui::DragValue::new(&mut selected_entrance.entrance_y)
                    .hexadecimal(4, false, true)
                    .range(0..=0xffff);
                ui.add(drag_value_y);
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
            // New exits have nil UUIDs but first (0) indexes, fix the UUIDs
            let _ = de.loaded_course.update_exit_uuids();
            de.course_settings.selected_exit = Some(new_uuid);
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
        }
        ui.add_enabled(false, egui::Button::new("Delete"));
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
                        let label = ui.label(exit.label.to_string());
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
            let selected_exit = de.get_selected_exit_mut();
            if selected_exit.is_none() {
                ui.label("No Exit selected");
                return;
            }
            let selected_exit = selected_exit.unwrap();
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
    ui.label(format!("Which Screen?: {:X}",which_screen));
    ui.label(format!("Entrance Animation?: {:X}",enter_map_anim));
}

fn show_exit_pos(ui: &mut egui::Ui, selected_exit: &mut MapExit) {
    ui.horizontal(|ui| {
        let drag_value_x = egui::DragValue::new(&mut selected_exit.exit_x)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        ui.add(drag_value_x);
        let drag_value_y = egui::DragValue::new(&mut selected_exit.exit_y)
            .hexadecimal(4, false, true)
            .range(0..=0xffff);
        ui.add(drag_value_y);
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

fn show_exit_target_map(ui: &mut egui::Ui, selected_exit: &mut MapExit, maps: &Vec<CourseMapInfo>) {
    let course = maps.iter().find(|x| x.uuid == selected_exit.target_map );
    if course.is_none() {
        log_write("Somehow, course was none", LogLevel::ERROR);
        return;
    }
    let course = course.unwrap();
    let selected_exit_stored = selected_exit.clone();
    let _exit_target_map_dropdown = egui::ComboBox::from_label("Target Map")
        .selected_text(course.label.clone())
        .show_ui(ui, |ui| {
            for map in maps {
                ui.selectable_value(&mut selected_exit.target_map, map.uuid, map.label.clone());
            }
        });
    if selected_exit_stored != *selected_exit {
        // The new entrance will be invalid! Reset it to the first one
        log_write("Selected exit target map changed", LogLevel::DEBUG);
        let course_new = maps.iter().find(|x| x.uuid == selected_exit.target_map );
        if course_new.is_none() {
            log_write("Failed to find course for selected exit target map", LogLevel::ERROR);
        }
        let course_new = course_new.unwrap();
        if course_new.map_entrances.is_empty() {
            log_write("New exit target map has no entrances",LogLevel::ERROR);
        }
        selected_exit.target_map_entrance = course_new.map_entrances[0].uuid;
    }
}

fn show_exit_target_entrance(ui: &mut egui::Ui, selected_exit: &mut MapExit, maps: &Vec<CourseMapInfo>) {
    let course = maps.iter().find(|x| x.uuid == selected_exit.target_map );
    if course.is_none() {
        log_write("Somehow, course was none", LogLevel::ERROR);
        return;
    }
    let course = course.unwrap();
    let cur_entrance = course.get_entrance(&selected_exit.target_map_entrance);
    if cur_entrance.is_none() {
        return;
    }
    let cur_entrance = cur_entrance.unwrap();
    let entrances = course.map_entrances.clone();
    let _exit_target_map_dropdown = egui::ComboBox::from_label("Target Entrance")
        .selected_text(cur_entrance.label.clone())
        .show_ui(ui, |ui| {
            for enter in entrances {
                ui.selectable_value(&mut selected_exit.target_map_entrance, enter.uuid, enter.label.clone());
            }
        });
}
