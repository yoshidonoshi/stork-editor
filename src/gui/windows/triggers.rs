
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use uuid::Uuid;

use crate::{data::area::{Trigger, TriggerData}, engine::displayengine::DisplayEngine};

pub fn show_triggers_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    StripBuilder::new(ui)
        .size(Size::exact(100.0))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                draw_trigger_list(ui, de);
            });
            strip.cell(|ui| {
                if !de.trigger_settings.selected_uuid.is_nil() {
                    draw_trigger_settings(ui, de, de.trigger_settings.selected_uuid);
                }
            });
        });
}

fn draw_trigger_list(ui: &mut egui::Ui, de: &mut DisplayEngine) {
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
            let area = de.loaded_map.get_area();
            if area.is_none() {
                return;
            }
            let triggers = &area.unwrap().triggers;
            for trigger in triggers {
                body.row(20.0, |mut row| {
                    let row_index = row.index();
                    row.set_selected(de.trigger_settings.selected_uuid == trigger.uuid);
                    row.col(|ui| {
                        let label = ui.label(format!("Trigger 0x{:X}",row_index));
                        if label.clicked() {
                            de.trigger_settings.selected_uuid = trigger.uuid;
                        }
                    });
                    if row.response().clicked() {
                        de.trigger_settings.selected_uuid = trigger.uuid;
                    }
                });
            }
        });
}

fn draw_trigger_settings(ui: &mut egui::Ui, de: &mut DisplayEngine, trigger_uuid: Uuid) {
    let t_get_res = de.loaded_map.get_area_mut();
    if t_get_res.is_none() {
        de.trigger_settings.selected_uuid = Uuid::nil();
        return;
    }
    let trigger_data: &mut TriggerData = t_get_res.unwrap();
    if trigger_data.triggers.is_empty() {
        return;
    }
    let triggers = &mut trigger_data.triggers;
    let mut t1 = triggers.iter_mut().filter(|x| x.uuid == trigger_uuid).collect::<Vec<&mut Trigger>>();
    let t = &mut t1[0];
    let trigger_before = t.clone();
    // Left X
    ui.horizontal(|ui| {
        let left_x = egui::DragValue::new(&mut t.left_x)
            .hexadecimal(4, false, true)
            .range(0..=(t.right_x-1));
        ui.label("Left X");
        ui.add(left_x);
    });
    // Top Y
    ui.horizontal(|ui| {
        let top_y = egui::DragValue::new(&mut t.top_y)
            .hexadecimal(4, false, true)
            .range(0..=(t.bottom_y-1));
        ui.label("Top Y");
        ui.add(top_y);
    });
    // Right X
    ui.horizontal(|ui| {
        let right_x = egui::DragValue::new(&mut t.right_x)
            .hexadecimal(4, false, true)
            .range((t.left_x+1)..=0xffff);
        ui.label("Right X");
        ui.add(right_x);
    });
    // Bottom Y
    ui.horizontal(|ui| {
        let bottom_y = egui::DragValue::new(&mut t.bottom_y)
            .hexadecimal(4, false, true)
            .range((t.top_y+1)..=0xffff);
        ui.label("Bottom Y");
        ui.add(bottom_y);
    });
    if **t != trigger_before {
        de.unsaved_changes = true;
    }
}
