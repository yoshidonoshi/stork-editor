
use egui::Color32;
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use uuid::Uuid;

use crate::{data::{area::{Trigger, TriggerData}, mapfile::TopLevelSegmentWrapper, types::CurrentLayer}, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

pub fn show_triggers_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    if de.display_settings.current_layer != CurrentLayer::Triggers {
        ui.disable();
    }
    if de.loaded_map.get_area().is_none() {
        let create = ui.button("Trigger database not found, create?");
        if create.clicked() {
            let t = TriggerData::default();
            de.loaded_map.segments.push(TopLevelSegmentWrapper::AREA(t));
            log_write("Created new AREA database", LogLevel::LOG);
            return;
        }
        ui.disable();
    }
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
        let add_button = ui.add(egui::Button::new("New"));
        if add_button.clicked() {
            log_write("Adding new Trigger", LogLevel::LOG);
            let area_mut_res = de.loaded_map.get_area_mut();
            if area_mut_res.is_none() {
                return;
            }
            let area = area_mut_res.unwrap();
            let new_trigger = Trigger { left_x: 2, top_y: 2, right_x: 12, bottom_y: 12, uuid: Uuid::new_v4() };
            de.trigger_settings.selected_uuid = new_trigger.uuid;
            area.triggers.push(new_trigger);
            de.unsaved_changes = true;
            de.graphics_update_needed = true;
        }
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
        let del = ui.add_enabled(de.trigger_settings.selected_uuid != Uuid::nil(),
            egui::Button::new("Delete"));
        if del.clicked() {
            log_write(format!("Attempting to delete Trigger {}",de.trigger_settings.selected_uuid), LogLevel::DEBUG);
            let area_mut_res = de.loaded_map.get_area_mut();
            if area_mut_res.is_none() {
                return;
            }
            let area = area_mut_res.unwrap();
            let _did_delete = area.delete(de.trigger_settings.selected_uuid);
            de.trigger_settings.selected_uuid = Uuid::nil();
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
        }
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
    let t1 = triggers.iter_mut().find(|x| x.uuid == trigger_uuid);
    if t1.is_none() {
        log_write(format!("Could not find Trigger with UUID '{}'",trigger_uuid), LogLevel::WARN);
        de.trigger_settings.selected_uuid = Uuid::nil();
        return;
    }
    let t = t1.unwrap();
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
    if *t != trigger_before {
        de.unsaved_changes = true;
    }
}
