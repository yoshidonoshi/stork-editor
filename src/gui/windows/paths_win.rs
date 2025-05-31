
use egui::Color32;
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use uuid::Uuid;

use crate::{data::{mapfile::TopLevelSegmentWrapper, path::{PathDatabase, PathLine, PathPoint}, types::CurrentLayer}, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

const CHANGE_RATE: u32 = 0x1000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PathAngle {
    pub x: i16,
    pub y: i16
}

pub fn show_paths_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    if de.display_settings.current_layer != CurrentLayer::Paths {
        ui.disable();
    }
    if de.loaded_map.get_path().is_none() {
        let create = ui.button("Path database not found, create?");
        if create.clicked() {
            let pd = PathDatabase::default();
            de.loaded_map.segments.push(TopLevelSegmentWrapper::PATH(pd));
            log_write("Create PATH database", LogLevel::LOG);
            return;
        }
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
        let btn_add = ui.add(egui::Button::new("New"));
        if btn_add.clicked() {
            log_write("Creating new PathLine", LogLevel::LOG);
            let Some(path) = de.loaded_map.get_path() else {
                de.path_settings.selected_line = Uuid::nil();
                de.path_settings.selected_point = Uuid::nil();
                return;
            };
            // Empty, but with a new UUID
            let mut new_blank_line = PathLine::default();
            new_blank_line.points.push(PathPoint::default()); // Don't let it be empty
            path.lines.push(new_blank_line);
            path.fix_term();
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
            log_write("New PathLine created", LogLevel::DEBUG);
        }
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
        let del = ui.add_enabled(!de.path_settings.selected_line.is_nil(), egui::Button::new("Delete"));
        if del.clicked() {
            let Some(path) = de.loaded_map.get_path() else {
                de.path_settings.selected_line = Uuid::nil();
                de.path_settings.selected_point = Uuid::nil();
                return;
            };
            let _ = path.delete_line(de.path_settings.selected_line);
            de.path_settings.selected_line = Uuid::nil();
            de.path_settings.selected_point = Uuid::nil();
            de.unsaved_changes = true;
            de.graphics_update_needed = true;
            path.fix_term();
            log_write("Line deleted", LogLevel::LOG);
        }
    });
    ui.add_space(5.0);
    let _table = TableBuilder::new(ui)
        .striped(true)
        .column(Column::exact(100.0))
        .sense(egui::Sense::click())
        .body(|mut body| {
            let Some(path) = de.loaded_map.get_path() else {
                return;
            };
            let paths: &mut Vec<PathLine> = &mut path.lines;
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
        let new_btn = ui.add(egui::Button::new("New"));
        if new_btn.clicked() {
            log_write("Creating PathPoint", LogLevel::DEBUG);
            let Some(path) = de.loaded_map.get_path() else {
                log_write("Cannot get PATH for point creation", LogLevel::ERROR);
                return;
            };
            // Now get the line
            let Some(line) = path.lines.iter_mut().find(|x| x.uuid == de.path_settings.selected_line) else {
                log_write("Cannot get Line for point creation", LogLevel::ERROR);
                return;
            };
            let new_point = PathPoint::default();
            line.points.push(new_point);
            de.unsaved_changes = true;
            de.graphics_update_needed = true;
            path.fix_term();
            log_write("PathPoint created", LogLevel::LOG);
        }
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;
        let del = ui.add_enabled(!de.path_settings.selected_point.is_nil(), egui::Button::new("Delete"));
        if del.clicked() {
            let Some(path) = de.loaded_map.get_path() else {
                log_write("Cannot get PATH for point deletion", LogLevel::ERROR);
                de.path_settings.selected_line = Uuid::nil();
                de.path_settings.selected_point = Uuid::nil();
                return;
            };
            // Now get the line
            let Some(line) = path.lines.iter_mut().find(|x| x.uuid == de.path_settings.selected_line) else {
                log_write("Cannot get Line for point deletion", LogLevel::ERROR);
                de.path_settings.selected_line = Uuid::nil();
                de.path_settings.selected_point = Uuid::nil();
                return;
            };
            if line.points.len() <= 1 {
                log_write("There can only be (at least) one (point)!", LogLevel::WARN);
                return;
            }
            let Some(point_pos) = line.points.iter().position(|x| x.uuid == de.path_settings.selected_point) else {
                log_write("Cannot get Point for point deletion", LogLevel::ERROR);
                de.path_settings.selected_point = Uuid::nil();
                return;
            };
            line.points.remove(point_pos);
            de.path_settings.selected_point = Uuid::nil();
            de.graphics_update_needed = true;
            de.unsaved_changes = true;
            path.fix_term();
            log_write("Point deleted", LogLevel::LOG);
        }
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
            let Some(path) = de.loaded_map.get_path() else { return };
            let paths = &mut path.lines;
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
    let Some(path_db) = de.loaded_map.get_path() else { return };
    let paths = &mut path_db.lines;
    if let Some(path) = paths.iter_mut().find(|x| x.uuid == de.path_settings.selected_line) {
        if let Some(point) = path.points.iter_mut().find(|y| y.uuid == de.path_settings.selected_point) {
            let point_before = point.clone();
            //ui.label("Warning: This section is WIP, red connecting line is not accurate");
            ui.horizontal(|ui| {
                let angle = egui::DragValue::new(&mut point.angle)
                    .speed(0x10)
                    .hexadecimal(5, false, true);
                ui.add(angle);
                ui.label("Angle");
            });
            ui.horizontal(|ui| {
                let distance = egui::DragValue::new(&mut point.distance)
                    .hexadecimal(4, false,true);
                ui.add(distance);
                ui.label("Distance");
            });
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
                path_db.fix_term();
                de.unsaved_changes = true;
                de.graphics_update_needed = true;
            }
        }
    }

}
