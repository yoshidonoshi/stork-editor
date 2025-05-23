use std::f32;

use egui::ScrollArea;
use egui_extras::{Column, Size, StripBuilder, TableBuilder};

use crate::{data::sprites::SpriteMetadata, utils::{log_write, settings_to_string, string_to_settings, LogLevel}};

use super::gui::Gui;

pub fn sprite_panel_show(ui: &mut egui::Ui, gui_state: &mut Gui) {
    StripBuilder::new(ui)
        .size(Size::exact(100.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                let sprites_len = gui_state.display_engine.selected_sprite_uuids.len();
                if sprites_len == 1 {
                    let sprite = &gui_state.display_engine.loaded_map
                        .get_sprite_by_uuid(gui_state.display_engine.selected_sprite_uuids[0])
                        .expect("Selected sprite UUID should exist on panel");
                    let sprite_meta = gui_state.sprite_metadata.get(&sprite.object_id);
                    if sprite_meta.is_none() {
                        log_write(format!("Failed to get sprite_meta for ID 0x{:X} on panel",&sprite.object_id), LogLevel::ERROR);
                        return;
                    }
                    let sprite_meta = sprite_meta.unwrap();
                    ui.label(format!("[0x{:03X}]: {}",&sprite.object_id,&sprite_meta.name));
                    ui.label(&sprite_meta.description);
                    ui.label(format!("X/Y Position: 0x{:X}/0x{:X}",&sprite.x_position,&sprite.y_position));
                    if sprite.settings_length != 0 {
                        ui.add(egui::TextEdit::multiline(&mut gui_state.display_engine.latest_sprite_settings).desired_width(120.0));
                        let res = ui.add_enabled(
                            is_settings_string_valid(
                                &gui_state.display_engine.latest_sprite_settings,
                                sprite.settings_length as usize
                            ) && gui_state.display_engine.latest_sprite_settings != settings_to_string(&sprite.settings),
                            egui::Button::new("Update Settings")
                        );
                        if res.clicked() {
                            log_write("Updating selected Sprite settings".to_owned(), LogLevel::LOG);
                            let poss_settings = string_to_settings(&gui_state.display_engine.latest_sprite_settings);
                            if poss_settings.is_err() {
                                log_write(format!("Still had bad settings somehow: '{}'",poss_settings.unwrap_err()), LogLevel::ERROR);
                            } else {
                                let new_settings = poss_settings.unwrap();
                                gui_state.display_engine.loaded_map.update_sprite_settings(sprite.uuid, new_settings);
                                gui_state.display_engine.unsaved_changes = true;
                                gui_state.display_engine.graphics_update_needed = true;
                            }
                        }
                    } else {
                        ui.label("No Settings");
                    }
                } else if sprites_len == 0 {
                    ui.label("No sprites selected");
                } else {
                    ui.label("Multiple sprites selected");
                }
            });
            strip.cell(|ui| {
                ui.separator();
                render_table(ui, gui_state);
            });
        });

}

fn is_settings_string_valid(settings_string: &String, ideal_len: usize) -> bool {
    let mut test_settings: Vec<u8> = Vec::new();
    let split: Vec<&str> = settings_string.split(' ').collect();
    for str8 in split {
        let pos_u8 = u8::from_str_radix(str8, 16);
        if pos_u8.is_err() {
            //log_write(format!("Could not format '{}' as u8, reason: '{}'",str8,pos_u8.unwrap_err()), LogLevel::DEBUG);
            return false;
        } else {
            let u8val = pos_u8.unwrap();
            test_settings.push(u8val);
        }
    }
    test_settings.len() == ideal_len
}

fn render_table(ui: &mut egui::Ui, gui_state: &mut Gui) {
    let row_height = 20.0;
    let sprite_count = &gui_state.display_engine.level_sprites.len();
    let sprite_metadata = gui_state.sprite_metadata.clone();
    ScrollArea::vertical().max_height(f32::INFINITY).show(ui, |ui| {
        let _table = TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(150.0))
            //.min_scrolled_height(0.0)
            .sense(egui::Sense::click())
            .body(|body| {
                body.heterogeneous_rows((0..*sprite_count).map(|_| row_height), |mut row| {
                    let index = row.index();
                    let cur_sprite = &gui_state.display_engine.level_sprites[index].clone();
                    if !sprite_metadata.contains_key(&cur_sprite.object_id) {
                        row.col(|ui| {
                            let missing_sprite = ui.label(format!("Missing metadata (0x{:X}, len {:X})",
                                &cur_sprite.object_id,&cur_sprite.settings_length));
                            if missing_sprite.clicked() {
                                log_write(format!("Could not get sprite metadata for object ID '0x{:X}'",&cur_sprite.object_id), LogLevel::ERROR);
                                log_write(format!("Settings length: 0x{:X}; data: {:?}",&cur_sprite.settings_length,&cur_sprite.settings), LogLevel::LOG);
                            }
                        });
                        return;
                    }
                    let sprite_meta: &SpriteMetadata = &sprite_metadata[&cur_sprite.object_id];
                    let (_,row_res) = row.col(|ui| {
                        if gui_state.display_engine.selected_sprite_uuids.contains(&cur_sprite.uuid) {
                            let res = ui.label(&sprite_meta.name)
                                .interact(egui::Sense::hover())
                                .interact(egui::Sense::click())
                                .highlight();
                            if res.hovered() {
                                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                            }
                            if res.clicked() {
                                gui_state.select_sprite_from_list(&index, &cur_sprite.uuid);
                            }
                        } else {
                            let res = ui.label(&sprite_meta.name)
                                .interact(egui::Sense::hover())
                                .interact(egui::Sense::click());
                            if res.hovered() {
                                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                            }
                            if res.clicked() {
                                gui_state.select_sprite_from_list(&index, &cur_sprite.uuid);
                            }
                        }
                    });
                    if row_res.clicked() {
                        gui_state.select_sprite_from_list(&index, &cur_sprite.uuid);
                    }
                });
            });
    });
}
