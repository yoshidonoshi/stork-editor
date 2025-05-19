
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use uuid::Uuid;

use crate::{data::types::CurrentLayer, engine::displayengine::DisplayEngine};

const CHANGE_RATE: u32 = 0x10000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PathAngle {
    pub x: i16,
    pub y: i16
}

pub fn show_paths_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    if de.display_settings.current_layer != CurrentLayer::PATHS {
        ui.disable();
    }
    StripBuilder::new(ui)
        .size(Size::exact(100.0))
        .size(Size::exact(100.0))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                draw_path_list(ui, de);
            });
            strip.cell(|ui| {
                draw_point_list(ui, de);
            });
            strip.cell(|ui| {
                draw_point_settings(ui, de);
            });
        });
}

fn draw_path_list(ui: &mut egui::Ui, de: &mut DisplayEngine) {
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
            let path_res = de.loaded_map.get_path();
            if path_res.is_none() {
                return;
            }
            let paths = &mut path_res.unwrap().lines;
            for path in paths {
                body.row(20.0, |mut row| {
                    let row_index = row.index();
                    row.set_selected(de.path_settings.selected_line == path.uuid);
                    row.col(|ui| {
                        let label = ui.label(format!("Path 0x{:X}",row_index));
                        if label.clicked() {
                            de.path_settings.selected_line = path.uuid;
                            de.path_settings.selected_point = Uuid::nil();
                        }
                    });
                    if row.response().clicked() {
                        de.path_settings.selected_line = path.uuid;
                        de.path_settings.selected_point = Uuid::nil();
                    }
                });
            }
        });
}

fn draw_point_list(ui: &mut egui::Ui, de: &mut DisplayEngine) {
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
            if de.path_settings.selected_line.is_nil() {
                return;
            }
            let path_res = de.loaded_map.get_path();
            if path_res.is_none() {
                return;
            }
            let paths = &mut path_res.unwrap().lines;
            if let Some(path) = paths.iter_mut().find(|x| x.uuid == de.path_settings.selected_line) {
                for point in &mut path.points {
                    body.row(20.0, |mut row| {
                        let row_index = row.index();
                        row.set_selected(de.path_settings.selected_point == point.uuid);
                        row.col(|ui| {
                            let label = ui.label(format!("Point 0x{:X}",row_index));
                            if label.clicked() {
                                de.path_settings.selected_point = point.uuid;
                            }
                        });
                        if row.response().clicked() {
                            de.path_settings.selected_point = point.uuid;
                        }
                    });
                }
            }
        });
}

fn draw_point_settings(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    if de.path_settings.selected_line.is_nil() {
        return;
    }
    if de.path_settings.selected_point.is_nil() {
        return;
    }
    let path_res = de.loaded_map.get_path();
    if path_res.is_none() {
        return;
    }
    let paths = &mut path_res.unwrap().lines;
    if let Some(path) = paths.iter_mut().find(|x| x.uuid == de.path_settings.selected_line) {
        if let Some(point) = path.points.iter_mut().find(|y| y.uuid == de.path_settings.selected_point) {
            let point_before = point.clone();
            ui.label("Warning: This section is WIP");
            // First is Angle
            if ui.button("Invert Angle").clicked() {
                point.angle *= -1;
            }
            if point.angle < 0 {
                ui.horizontal(|ui| {
                    let angle_drag = egui::DragValue::new(&mut point.angle)
                        .hexadecimal(4, false, true)
                        .range(i16::MIN..=-1);
                    ui.label("Neg. Angle");
                    ui.add(angle_drag);
                });
            } else {
                ui.horizontal(|ui| {
                    let angle_drag = egui::DragValue::new(&mut point.angle)
                        .hexadecimal(4, false, true)
                        .range(0..=i16::MAX);
                    ui.label("Pos. Angle");
                    ui.add(angle_drag);
                });
            }
            // Next is distance
            if ui.button("Invert distance").clicked() {
                point.distance *= -1;
            }
            if point.distance >= 0 {
                ui.horizontal(|ui| {
                    let distance_drag = egui::DragValue::new(&mut point.distance)
                        .hexadecimal(4, false, true)
                        .range(0..=u32::MAX);
                    ui.label("Pos. Distance");
                    ui.add(distance_drag);
                });
            } else {
                ui.horizontal(|ui| {
                    let distance_drag = egui::DragValue::new(&mut point.distance)
                        .hexadecimal(4, false, true)
                        .range(i32::MIN..=-1);
                    ui.label("Neg. Distance");
                    ui.add(distance_drag);
                });
            }
            // Then X and Y
            ui.horizontal(|ui| {
                let x_drag = egui::DragValue::new(&mut point.x_fine)
                    .hexadecimal(8, false, true)
                    .speed(CHANGE_RATE)
                    .range(0..=u32::MAX);
                ui.label("X (Fine)");
                ui.add(x_drag);
            });
            ui.horizontal(|ui| {
                let y_drag = egui::DragValue::new(&mut point.y_fine)
                    .hexadecimal(8, false, true)
                    .speed(CHANGE_RATE)
                    .range(0..=u32::MAX);
                ui.label("Y (Fine)");
                ui.add(y_drag);
            });
            if point_before != *point {
                de.unsaved_changes = true;
                de.graphics_update_needed = true;
            }
        }
    }

}
