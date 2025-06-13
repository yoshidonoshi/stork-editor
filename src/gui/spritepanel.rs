use std::f32;

use egui::ScrollArea;
use egui_extras::{Column, Size, StripBuilder, TableBuilder};

use crate::{data::sprites::{LevelSprite, SpriteMetadata}, gui::{spritesettings, SpriteSettings}, load::SPRITE_METADATA, utils::{self, bytes_to_hex_string, is_debug, log_write, string_to_settings, LogLevel}, NON_MAIN_FOCUSED};

use super::gui::Gui;

pub fn sprite_panel_show(ui: &mut egui::Ui, gui_state: &mut Gui) {
    puffin::profile_function!();
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
                    let Some(sprite_meta) = SPRITE_METADATA.get(&sprite.object_id) else {
                        log_write(format!("Failed to get sprite_meta for ID 0x{:X} on panel",&sprite.object_id), LogLevel::Error);
                        return;
                    };
                    ui.label(format!("[0x{:03X}]: {}",&sprite.object_id,&sprite_meta.name));
                    ui.label(&sprite_meta.description);
                    ui.label(format!("X/Y Position: 0x{:X}/0x{:X}",&sprite.x_position,&sprite.y_position));
                    if sprite.settings_length != 0 {
                        #[allow(clippy::manual_range_patterns)]
                        match sprite.object_id {
                            0x23 => {
                                let mut pipe = spritesettings::GreenPipe::from_sprite(sprite);
                                pipe.show_ui(ui);
                                let comp = pipe.compile();
                                settings_save_check(gui_state, comp, sprite);
                            }
                            0x36 | 0x37 | 0x38 | 0x39 => {
                                let mut shyguy = spritesettings::ShyGuy::from_sprite(sprite);
                                shyguy.show_ui(ui);
                                let comp = shyguy.compile();
                                settings_save_check(gui_state, comp, sprite);
                            }
                            0x9A => {
                                let mut red_arrow_sign = spritesettings::RedArrowSign::from_sprite(sprite);
                                red_arrow_sign.show_ui(ui);
                                let comp = red_arrow_sign.compile();
                                settings_save_check(gui_state, comp, sprite);
                            }
                            0x9F => {
                                let mut hint_block = spritesettings::HintBlock::from_sprite(sprite);
                                hint_block.show_ui(ui);
                                let comp = hint_block.compile();
                                settings_save_check(gui_state, comp, sprite);
                            }
                            _ => { // Anything we don't know
                                let ml = ui.add(egui::TextEdit::multiline(&mut gui_state.display_engine.latest_sprite_settings).desired_width(120.0));
                                if ml.has_focus() {
                                    *NON_MAIN_FOCUSED.lock().unwrap() = true;
                                }
                                let res = ui.add_enabled(
                                    is_settings_string_valid(
                                        &gui_state.display_engine.latest_sprite_settings,
                                        sprite.settings_length as usize
                                    ) && gui_state.display_engine.latest_sprite_settings != bytes_to_hex_string(&sprite.settings),
                                    egui::Button::new("Update Settings")
                                );
                                if res.clicked() {
                                    log_write("Updating selected Sprite settings".to_owned(), LogLevel::Log);
                                    match string_to_settings(&gui_state.display_engine.latest_sprite_settings) {
                                        Err(error) => log_write(format!("Still had bad settings somehow: '{error}'"), LogLevel::Error),
                                        Ok(new_settings) => {
                                            gui_state.display_engine.loaded_map.update_sprite_settings(sprite.uuid, new_settings);
                                            gui_state.display_engine.unsaved_changes = true;
                                            gui_state.display_engine.graphics_update_needed = true;
                                        }
                                    };
                                }
                            } // End unknown settings
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

fn is_settings_string_valid(settings_string: &str, ideal_len: usize) -> bool {
    let mut test_settings: Vec<u8> = Vec::new();
    let split: Vec<&str> = settings_string.split(' ').collect();
    for str8 in split {
        let Ok(u8val) = u8::from_str_radix(str8, 16) else { return false };
        test_settings.push(u8val);
    }
    test_settings.len() == ideal_len
}

fn render_table(ui: &mut egui::Ui, gui_state: &mut Gui) {
    let row_height = 20.0;
    let sprite_count = &gui_state.display_engine.level_sprites.len();
    ScrollArea::vertical().max_height(f32::INFINITY).show(ui, |ui| {
        let _table = TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(145.0))
            //.min_scrolled_height(0.0)
            .sense(egui::Sense::click())
            .body(|body| {
                body.heterogeneous_rows((0..*sprite_count).map(|_| row_height), |mut row| {
                    let index = row.index();
                    let cur_sprite = gui_state.display_engine.level_sprites[index].clone();
                    if !SPRITE_METADATA.contains_key(&cur_sprite.object_id) {
                        row.col(|ui| {
                            let missing_sprite = ui.label(format!("Missing metadata (0x{:X}, len {:X})",
                                &cur_sprite.object_id,&cur_sprite.settings_length));
                            if missing_sprite.clicked() {
                                log_write(format!("Could not get sprite metadata for object ID '0x{:X}'",&cur_sprite.object_id), LogLevel::Error);
                                log_write(format!("Settings length: 0x{:X}; data: {:?}",&cur_sprite.settings_length,&cur_sprite.settings), LogLevel::Log);
                            }
                        });
                        return;
                    }
                    let sprite_meta: &SpriteMetadata = &SPRITE_METADATA[&cur_sprite.object_id];
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

fn settings_save_check(gui_state: &mut Gui, comp: Vec<u8>, sprite: &LevelSprite) {
    if *comp != sprite.settings {
        if is_debug() {
            log_write("Settings before and after:", LogLevel::Debug);
            utils::print_vector_u8(&sprite.settings);
            utils::print_vector_u8(&comp);
        }
        gui_state.display_engine.unsaved_changes = true;
        gui_state.display_engine.graphics_update_needed = true;
        gui_state.display_engine.loaded_map.update_sprite_settings(sprite.uuid, comp);
    }
}
