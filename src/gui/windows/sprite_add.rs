use egui::{Hyperlink, ScrollArea};
use egui_extras::{Column, TableBuilder};

use crate::{data::types::CurrentLayer, engine::displayengine::DisplayEngine, load::SPRITE_METADATA, NON_MAIN_FOCUSED};

pub fn sprite_add_window_show(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    ui.add(Hyperlink::from_label_and_url("Sprite Documentation", env!("SPRITE_DOC")));
    if de.display_settings.current_layer != CurrentLayer::Sprites {
        ui.disable();
    }
    let search_bar = ui.text_edit_singleline(&mut de.sprite_search_query);
    if search_bar.has_focus() {
        *NON_MAIN_FOCUSED.lock().unwrap() = true;
    }
    ScrollArea::vertical()
        .auto_shrink(false)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(ui, |ui| {
            create_table(ui, de, &de.sprite_search_query.trim().to_lowercase());
        });
}

fn create_table(ui: &mut egui::Ui, de: &mut DisplayEngine, query: &str) {
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
            let sprite_meta = SPRITE_METADATA.get(&sprite_index);
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
                    row.set_selected(sprite_index == de.selected_sprite_to_place.unwrap_or(0xffff));
                    // ID
                    row.col(|ui| {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        let res = ui.label(format!("0x{:03X}",sprite.sprite_id));
                        if res.clicked() {
                            de.selected_sprite_to_place = Some(sprite_index);
                        }
                    });
                    // Name
                    row.col(|ui| {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        let res = ui.label(&sprite.name);
                        if res.clicked() {
                            de.selected_sprite_to_place = Some(sprite_index);
                        }
                    });
                    // Description
                    row.col(|ui| {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        let res = ui.label(&sprite.description);
                        if res.clicked() {
                            de.selected_sprite_to_place = Some(sprite_index);
                        }
                    });
                    if row.response().clicked() {
                        de.selected_sprite_to_place = Some(sprite_index);
                    }
                });

            }
        }
    });
}
