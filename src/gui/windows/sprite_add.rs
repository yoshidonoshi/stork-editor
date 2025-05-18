use std::collections::HashMap;

use egui::{Hyperlink, ScrollArea};
use egui_extras::{Column, TableBuilder};

use crate::{data::sprites::SpriteMetadata, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

pub fn sprite_add_window_show(ui: &mut egui::Ui, de: &mut DisplayEngine, meta: &HashMap<u16,SpriteMetadata>) {
    ui.add(Hyperlink::from_label_and_url("Sprite Documentation", env!("SPRITE_DOC")));
    let _search_bar = ui.text_edit_singleline(&mut de.sprite_search_query);
    ScrollArea::vertical()
        .auto_shrink(false)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(ui, |ui| {
            create_table(ui, de, meta, &de.sprite_search_query.clone().trim().to_lowercase().to_string());
        });
}

fn create_table(ui: &mut egui::Ui, de: &mut DisplayEngine, meta: &HashMap<u16,SpriteMetadata>, query: &String) {
    let _table = TableBuilder::new(ui)
    .striped(true)
    .resizable(false)
    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
    .column(Column::exact(50.0))
    .column(Column::exact(150.0))
    .column(Column::exact(200.0))
    .sense(egui::Sense::click())
    .body(|mut body| {
        let max: u16 = 0x140;
        for sprite_index in 0..max {
            let sprite_meta = meta.get(&sprite_index);
            if let Some(sprite) = sprite_meta {
                if sprite.name == "Null" {
                    continue;
                }
                if !query.is_empty() {
                    // Filter
                    let mut show = false;
                    if sprite.name.to_lowercase().contains(query) {
                        show = true;
                    }
                    if sprite.description.to_lowercase().contains(query) {
                        show = true;
                    }
                    if !show {
                        continue;
                    }
                }
                body.row(20.0, |mut row| {
                    // ID
                    row.col(|ui| {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        let res = ui.label(format!("0x{:03X}",sprite.sprite_id));
                        if res.clicked() {
                            add_sprite(de, meta, sprite.sprite_id);
                        }
                    });
                    // Name
                    row.col(|ui| {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        let res = ui.label(format!("{}",sprite.name));
                        if res.clicked() {
                            add_sprite(de, meta, sprite.sprite_id);
                        }
                    });
                    // Description
                    row.col(|ui| {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        let res = ui.label(format!("{}",sprite.description));
                        if res.clicked() {
                            add_sprite(de, meta, sprite.sprite_id);
                        }
                    });
                    if row.response().clicked() {
                        add_sprite(de, meta, sprite.sprite_id);
                    }
                });

            }
        }
    });
}

fn add_sprite(de: &mut DisplayEngine, meta: &HashMap<u16,SpriteMetadata>, sprite_id: u16) {
    log_write(format!("Adding sprite with ID 0x{:X} to vrect corner",sprite_id), LogLevel::LOG);
    let sprite_meta = meta.get(&sprite_id).expect("Sprite Metadata should exist in add_sprite");
    let uuid = de.loaded_map.add_sprite_for_centering(sprite_id, vec![0;sprite_meta.default_settings_len as usize]);
    de.selected_sprite_uuids.clear();
    de.selected_sprite_uuids.push(uuid);
    de.graphics_update_needed = true;
    de.unsaved_changes = true;
}
