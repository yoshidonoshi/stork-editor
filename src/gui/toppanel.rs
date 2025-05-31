use crate::{data::{course_file::CourseInfo, mapfile::MapData, types::CurrentLayer}, utils::{self, log_write, LogLevel}};

use super::gui::Gui;
use egui::Button;
use strum::IntoEnumIterator;

pub fn top_panel_show(ui: &mut egui::Ui, gui_state: &mut Gui) {
    puffin::profile_function!();
    ui.horizontal(|ui| {
        // File Menu //
        //ui.spacing_mut().item_spacing = vec2(16.0,16.0);
        ui.menu_button("File", |ui| {
            let button_open_rom = ui.add_enabled(true, Button::new("Open ROM..."));
            if button_open_rom.clicked() {
                ui.close_menu();
                if let Err(error) = gui_state.do_open_rom() {
                    gui_state.do_alert(&error.cause);
                }
            }
            let button_open_project = ui.add_enabled(true, Button::new("Open Project..."));
            if button_open_project.clicked() {
                ui.close_menu();
                gui_state.do_open_project();
            }
            ui.separator();
            let button_change_course = ui.add_enabled(gui_state.project_open, Button::new("Change Course"));
            if button_change_course.clicked() {
                ui.close_menu();
                gui_state.do_change_course();
            }
            let button_change_map = ui.add_enabled(gui_state.project_open, Button::new("Select Map"));
            if button_change_map.clicked() {
                gui_state.do_change_map();
                ui.close_menu();
            }
            ui.separator();
            let button_save = ui.add_enabled(gui_state.project_open, Button::new("Save"));
            if button_save.clicked() {
                ui.close_menu();
                gui_state.do_save();
            }
            let button_export = ui.add_enabled(gui_state.project_open, Button::new("Export..."));
            if button_export.clicked() {
                ui.close_menu();
                gui_state.do_export();
            }
            ui.separator();
            let button_project_settings = ui.add_enabled(gui_state.project_open, Button::new("Settings"));
            if button_project_settings.clicked() {
                ui.close_menu();
                gui_state.settings_open = true;
            }
            let button_close_project = ui.add_enabled(gui_state.project_open, Button::new("Close Project"));
            if button_close_project.clicked() {
                ui.close_menu();
                gui_state.clear_map_data();
                gui_state.display_engine.loaded_map = MapData::default();
                gui_state.display_engine.loaded_course = CourseInfo::default();
                gui_state.project_open = false;
                gui_state.display_engine.game_version = None;
            }
            let button_quit = ui.button("Quit");
            if button_quit.clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        // Edit Menu //
        ui.menu_button("Edit", |ui| {
            if !gui_state.project_open {
                ui.disable();
            }
            let has_undos = gui_state.undoer.has_undo(&gui_state.display_engine.loaded_map);
            let button_undo = ui.add_enabled(has_undos, Button::new("Undo"));
            if button_undo.clicked() {
                ui.close_menu();
                gui_state.do_undo();
            }
            let has_redos = gui_state.undoer.has_redo(&gui_state.display_engine.loaded_map);
            let button_redo = ui.add_enabled(has_redos, Button::new("Redo"));
            if button_redo.clicked() {
                ui.close_menu();
                gui_state.do_redo();
            }
            ui.separator();
            let button_cut = ui.add_enabled(gui_state.is_cut_possible(), Button::new("Cut"));
            if button_cut.clicked() {
                ui.close_menu();
                gui_state.do_cut();
            }
            let button_copy = ui.add_enabled(gui_state.is_copy_possible(), Button::new("Copy"));
            if button_copy.clicked() {
                ui.close_menu();
                gui_state.do_copy();
            }
            let button_paste = ui.add_enabled(gui_state.is_paste_possible(), Button::new("Paste"));
            if button_paste.clicked() {
                ui.close_menu();
                gui_state.do_paste();
            }
            ui.separator();
            let button_select_all = ui.button("Select All");
            if button_select_all.clicked() {
                ui.close_menu();
                gui_state.do_select_all();
            }
            let button_select_none = ui.button("Select None");
            if button_select_none.clicked() {
                ui.close_menu();
                gui_state.do_select_none();
            }
            ui.separator();
            let button_clear = ui.button("Clear Layer");
            if button_clear.clicked() {
                gui_state.clear_modal_open = true;
                ui.close_menu();
            }
            let button_resize = ui.button("Resize layer");
            if button_resize.clicked() {
                if gui_state.display_engine.display_settings.is_cur_layer_bg() {
                    gui_state.resize_settings.reset_needed = true;
                    gui_state.resize_settings.window_open = true;
                    ui.close_menu();
                } else if gui_state.display_engine.display_settings.current_layer == CurrentLayer::Collision {
                    if let Some(colz_layer) = gui_state.display_engine.loaded_map.get_bg_with_colz() {
                        gui_state.do_alert(&format!("Cannot resize collision, as it is attached to the layer '{colz_layer}'"));
                    } else {
                        log_write("Could not get COLZ layer when attempting to open resize modal", LogLevel::DEBUG);
                    }
                    ui.close_menu();
                } else {
                    let cur_layer = gui_state.display_engine.display_settings.current_layer;
                    gui_state.do_alert(&format!("Cannot resize on layer '{:?}', dimensions controlled by BG layers",cur_layer));
                }
            }
        });
        // View Menu //
        ui.menu_button("View", |ui| {
            ui.disable();
            let _button_zoom_in = ui.button("Zoom In");
            let _button_zoom_out = ui.button("Zoom Out");
            ui.separator();
            let _button_close_windows = ui.button("Close Windows");
            let _button_sort_windows = ui.button("Sort Windows");
        });
        // Help Menu //
        ui.menu_button("Help", |ui| {
            let button_about = ui.button("About");
            if button_about.clicked() {
                gui_state.about_modal_open = true;
                ui.close_menu();
            }
            let button_report = ui.button("Report Bug");
            if button_report.clicked() {
                gui_state.bug_report_modal_open = true;
                ui.close_menu();
            }
            let button_help = ui.button("Help");
            if button_help.clicked() {
                gui_state.help_modal_open = true;
                ui.close_menu();
            }
            if utils::is_debug() {
                if ui.button("Enable profiling").clicked() {
                    utils::profile::enable_profiling();
                }
            }
        });
    }); // End top menu bar

    ui.horizontal(|ui|{
        ui.label("Layer").on_hover_ui(|ui|{
            ui.label("This dropdown determines what layer to work with, and locks the rest");
        });
        let selected_bg: &mut CurrentLayer = &mut gui_state.display_engine.display_settings.current_layer;
        let old_selected_bg = selected_bg.clone();
        let _cur_layer_combo = egui::ComboBox::from_label("")
            .selected_text(format!("{selected_bg:?}"))
            .show_ui(ui, |ui| {
                for layer in CurrentLayer::iter() {
                    ui.selectable_value(selected_bg, layer, format!("{layer:?}"));
                }
            });
        if *selected_bg != old_selected_bg {
            log_write("Cleaning up due to layer change", LogLevel::DEBUG);
            gui_state.display_engine.brush_settings.cur_selected_brush = Option::None;
            gui_state.display_engine.current_brush.clear();
            gui_state.display_engine.clipboard.bg_clip.clear();
            gui_state.display_engine.bg_sel_data.clear();
        }
        egui::ComboBox::new(egui::Id::new("visible_layers_drop"), "")
            .selected_text("Visible layers")
            .show_ui(ui, |ui| {
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_col, "Collision");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_sprites, "Sprites");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_bg1, "BG 1");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_bg2, "BG 2");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_bg3, "BG 3");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_paths, "Paths");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_triggers, "Triggers");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_entrances, "Entrances");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_exits, "Exits");
                ui.checkbox(&mut gui_state.display_engine.display_settings.show_breakable_rock, "Soft Rock Back");
            });
        let x = gui_state.display_engine.tile_hover_pos.x as u16;
        let y = gui_state.display_engine.tile_hover_pos.y as u16;
        ui.label(format!("Tile x/y: {:04X}/{:04X}",x,y));
    });
}
