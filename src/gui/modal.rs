use std::sync::{Arc, LazyLock};

use egui::{mutex::Mutex, Context, Hyperlink, Id, Modal, ProgressBar, ScrollArea, Widget};
use paste::paste;

use crate::{gui::{gui::Gui, windows::resize::show_resize_modal}, utils::{self, log_write, LogLevel}, VERSION};

#[macro_export]
macro_rules! modals {
    {$($name:ident $struct:tt)*} => {
        paste!{
            $(
                #[derive(Debug, Default)]
                pub struct [<$name:camel Data>] $struct
                pub static [<$name:snake:upper _MODAL>]: LazyLock<Arc<Mutex<Option<[<$name:camel Data>]>>>> = LazyLock::new(|| Arc::new(Mutex::new(None)));
            )*
        }
    };
    {$ctx:ident, $($name:ident |$data:ident, $close:ident, $ui:ident| $func:block)*} => {
        paste!{
            $(
                let mut close = false;
                #[allow(unused_mut)]
                let mut $close = || close = true;
                let mut mutex = [<$name:snake:upper _MODAL>].lock();
                #[allow(unused_variables)]
                if let Some($data) = &mut *mutex {
                    Modal::new(Id::new(concat!(stringify!($name),"_modal")))
                    .show($ctx, |$ui| {
                        $func;
                    });
                }
                if close {
                    *mutex = None;
                }
            )*
        }
    };
}

modals!{
    Resize {
        pub new_width: u16,
        pub new_height: u16,
        pub reset_needed: bool,
    }
    Alert {
        pub alert: String,
    }
    
    CloseChanges {}
    ExportChange {}
    Exporting {
        pub exporting_to: String,
        pub progress: f32,
    }
    Saving {
        pub quit_when_done: bool,
        pub export_after_done: bool,
        pub progress: f32,
    }
    
    CourseChanges {}
    MapChanges {}
    ChangeMap {}
    ChangeCourse {
        pub change_level_world_index: u32,
        pub change_level_level_index: u32,
    }
    
    About {}
    BugReport {}

    Clear {}
    Help {}

    AddMap {}
}

impl Gui {
    pub fn show_modals(&mut self, ctx: &Context) {
        modals!{
            ctx,
            Resize |data, close, ui| {
                if show_resize_modal(ui, &mut self.display_engine, data) {
                    close();
                }
            }
            Alert |data, close, ui| {
                ui.set_width(200.0);
                ui.heading("Alert");
                ui.label(data.alert.as_str());
                if ui.button("Okay").clicked() {
                    close();
                }
            }
            CloseChanges |data, close, ui| {
                ui.set_width(200.0);
                ui.heading("Save Changes?");
                ui.label("You have unsaved changes, do you want to save before you exit?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close();
                    }
                    if ui.button("Discard").clicked() {
                        close();
                        self.display_engine.unsaved_changes = false; // So it can actually close
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Save").clicked() {
                        *SAVING_MODAL.lock() = Some(SavingData {
                            progress: 0.0,
                            quit_when_done: true,
                            ..Default::default()
                        });
                    }
                });
            }
            ExportChange |data, close, ui| {
                ui.set_width(200.0);
                ui.heading("Save Changes?");
                ui.label("You have unsaved changes, do you want to save before export?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close();
                    }
                    if ui.button("Continue").clicked() {
                        self.do_export(true);
                        close();
                    }
                    if ui.button("Save and Continue").clicked() {
                        *SAVING_MODAL.lock() = Some(SavingData {
                            export_after_done: true,
                            progress: 0.0,
                            ..Default::default()
                        });
                        close();
                    }
                });
            }
            Exporting |data, close, ui| {
                let current_progress = data.progress;
                ui.set_width(200.0);
                ui.heading("Exporting ROM...");
                ui.label("This may take time, please wait");
                ProgressBar::new(current_progress).ui(ui);
                data.progress += 0.1;
                ctx.request_repaint();
                if current_progress == 0.4 {
                    // Do the actaul export here
                    self.export_rom_file(data.exporting_to.clone());
                }
                if current_progress >= 1.0 {
                    close();
                }
            }
            Saving |data, close, ui| {
                let saving_progress = &mut data.progress;
                ui.set_width(70.0);
                ui.heading("Saving...");
                ProgressBar::new(*saving_progress).ui(ui);
                if *saving_progress == 0.0 {
                    ctx.request_repaint();
                }
                if *saving_progress == 0.4 {
                    self.save_map();
                    self.save_course();
                }
                if *saving_progress >= 1.0 {
                    close();
                    self.display_engine.unsaved_changes = false;
                    if data.quit_when_done {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if data.export_after_done {
                        data.export_after_done = false;
                        self.do_export(false);
                    }
                } else {
                    *saving_progress = *saving_progress + 0.2;
                }
            }
            CourseChanges |data, close, ui| {
                ui.set_width(200.0);
                ui.heading("Save Changes?");
                ui.label("You have unsaved changes, do you want to save before changing Course?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close();
                    }
                    if ui.button("Continue").clicked() {
                        close();
                        *CHANGE_COURSE_MODAL.lock() = Some(ChangeCourseData { ..Default::default() });
                    }
                    if ui.button("Save and Continue").clicked() {
                        close();
                        *CHANGE_COURSE_MODAL.lock() = Some(ChangeCourseData { ..Default::default() });
                        self.do_save();
                    }
                });
            }
            MapChanges |data, close, ui| {
                ui.set_width(200.0);
                ui.heading("Save Changes?");
                ui.label("You have unsaved changes, do you want to save before changing map?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close();
                    }
                    if ui.button("Continue").clicked() {
                        close();
                        *CHANGE_MAP_MODAL.lock() = Some(Default::default());
                    }
                    if ui.button("Save and Continue").clicked() {
                        close();
                        *CHANGE_MAP_MODAL.lock() = Some(Default::default());
                        self.do_save();
                    }
                });
            }
            ChangeMap |data, close, ui| {
                ui.heading("Select map");
                ui.set_width(150.0);

                let crsb = self.display_engine.loaded_course.level_map_data.clone();
                ScrollArea::vertical().show(ui, |ui| {
                    for (map_index, map) in crsb.iter().enumerate() {
                        let mut but = ui.button(&map.map_filename_noext);
                        if map.map_filename_noext == self.display_engine.loaded_map.map_name {
                            but = but.highlight();
                        }
                        if but.clicked() {
                            // Since the targeting is done via GUI, but accesses the saved data
                            self.save_course();
                            // TODO: This is to be used once support for ALL map selection is working
                            // self.map_change_selected_map = map.map_filename_noext.clone();
                            self.change_map(map_index as u32);
                            close();
                        }
                    }
                });
            }
            ChangeCourse |data, close, ui| {
                ui.heading("Select a Course");
                ui.set_width(150.0);
                // World Selection //
                let _combo_world = egui::ComboBox::new(
                    egui::Id::new("change_level_world"), "World")
                    .selected_text(format!("{}",data.change_level_world_index+1))
                    .show_ui(ui, |ui| {
                        for x in 0..5_u32 {
                            ui.selectable_value(&mut data.change_level_world_index, x, (x+1).to_string());                          
                        }
                    });
                let _combo_level = egui::ComboBox::new(
                    egui::Id::new("change_level_level"), "Level")
                    .selected_text(format!("{}",data.change_level_level_index+1))
                    .show_ui(ui, |ui| {
                        for y in 0..10_u32 {
                            ui.selectable_value(&mut data.change_level_level_index, y, (y+1).to_string());
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close();
                    }
                    if ui.button("Okay").clicked() {
                        close();
                        self.change_level(data.change_level_world_index, data.change_level_level_index);
                    }
                });
            }
            About |data, close, ui| {
                ui.heading(format!("Stork {}",VERSION));
                ui.label("A ROM-hacking tool for Yoshi's Island DS");
                ui.label("Created by YoshiDonoshi/Zolarch");
                ui.add(Hyperlink::from_label_and_url("Source Code", env!("GITHUB_REPO")));
                ui.vertical_centered(|ui| {
                    let about_close_button = ui.button("Close");
                    if about_close_button.clicked() {
                        close();
                    }
                });
            }
            BugReport |data, close, ui| {
                ui.heading("Report a Bug");
                ui.label("The best place to report a bug or request features is on the Github:");
                ui.hyperlink(env!("GITHUB_REPO"));
                ui.label(format!("Please include your stork.log and version ({})",VERSION));
                ui.label("You can do the same on Discord, with more timely help and answers:");
                ui.hyperlink(env!("DISCORD"));
                ui.label("If those links has stopped working, find the thread here:");
                ui.hyperlink(env!("SMWC_FORUM"));
                ui.label("Thanks for helping to improve this tool!");
                ui.vertical_centered(|ui| {
                    let bug_report_close_button = ui.button("Close");
                    if bug_report_close_button.clicked() {
                        close();
                    }
                });
            }
            Clear |data, close, ui| {
                ui.heading("Clear Layer");
                ui.label(format!("This will delete everything on the current layer ({:?})",&self.display_engine.display_settings.current_layer));
                ui.label("Are you sure?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close();
                    }
                    if ui.button("Clear Layer").clicked() {
                        self.do_clear_layer();
                        close();
                    }
                });
            }
            Help |data, close, ui| {
                ui.heading("Help");
                ui.label("First, check out the documentation and FAQ:");
                ui.hyperlink(env!("DOC_URL"));
                ui.label("If you're still having trouble, ask a question on the Discord server:");
                ui.hyperlink(env!("DISCORD"));
                ui.vertical_centered(|ui| {
                    if ui.button("Close").clicked() {
                        close();
                    }
                });
            }
            AddMap |data, close, ui| {
                ui.heading("Choose a Map template");
                egui::ComboBox::new(egui::Id::new("add_map_combo_box"), "")
                    .selected_text(&self.display_engine.course_settings.add_map_selected)
                    .show_ui(ui, |ui| {
                        let mut map_keys: Vec<String> = self.display_engine.course_settings.map_templates.keys().cloned().collect();
                        map_keys.sort();
                        for map_name in map_keys {
                            ui.selectable_value(&mut self.display_engine.course_settings.add_map_selected,
                                map_name.clone(), &map_name);
                        }
                    }
                );
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close();
                    }
                    if ui.button("Add").clicked() {
                        let level = self.display_engine.course_settings.map_templates.get(
                            &self.display_engine.course_settings.add_map_selected);
                        let Some(level_file) = level else {
                            log_write(format!("Map template key not found: '{}'",
                                self.display_engine.course_settings.add_map_selected), LogLevel::Warn);
                            return;
                        };
                        let Some(template_path) = utils::get_template_folder(&self.export_directory) else {
                            log_write("Failed to get template directory", LogLevel::Error);
                            return;
                        };
                        self.display_engine.loaded_course.add_template(level_file, &template_path);
                        close();
                        self.display_engine.unsaved_changes = true;
                        self.display_engine.graphics_update_needed = true;
                    }
                });
            }
        }
    }
}
