use std::sync::{Arc, LazyLock};

use egui::{mutex::Mutex, Context, Id, Modal, ProgressBar, Widget};
use paste::paste;

use crate::gui::{gui::Gui, windows::resize::show_resize_modal};

#[macro_export]
macro_rules! modals {
    {$($name:ident $struct:tt)*} => {
        paste!{
            $(
                pub struct [<$name:camel Data>] $struct
                pub static [<$name:snake:upper _MODAL>]: LazyLock<Arc<Mutex<Option<[<$name:camel Data>]>>>> = LazyLock::new(|| Arc::new(Mutex::new(None)));
            )*
        }
    };
    {$($name:ident |$data:ident, $mutex:ident| $func:block)*} => {
        paste!{
            $(
                let mut $mutex = [<$name:snake:upper _MODAL>].lock();
                #[allow(unused_variables)]
                if let Some($data) = &mut *$mutex {
                    $func;
                }
                drop($mutex);
            )*
        }
    };
}

modals!{
    Resize {}
    Alert {
        pub alert: String,
    }
    CloseChanges {}
    ExportChange {}
    Exporting {
        pub progress: f32,
    }
    // RenameModal(String)
}

impl Gui {
    pub fn show_modals(&mut self, ctx: &Context) {
        modals!{
            Resize |data, mutex| {
                Modal::new(Id::new("resize_modal"))
                .show(ctx, |ui| {
                    show_resize_modal(ui, &mut self.display_engine, &mut self.resize_settings);
                });
            }
            Alert |alert_data, alert_mutex| {
                let alert_modal = Modal::new(Id::new("alert_modal"))
                .show(ctx, |ui| {
                    ui.set_width(200.0);
                    ui.heading("Alert");
                    ui.label(alert_data.alert.as_str());
                    ui.button("Okay").clicked()
                });
                if alert_modal.inner {
                    *alert_mutex = None;
                }
            }
            CloseChanges |cc_data, cc_mutex| {
                Modal::new(Id::new("close_changes_modal"))
                .show(ctx, |ui| {
                    ui.set_width(200.0);
                    ui.heading("Save Changes?");
                    ui.label("You have unsaved changes, do you want to save before you exit?");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            *cc_mutex = None;
                        }
                        if ui.button("Discard").clicked() {
                            *cc_mutex = None;
                            self.display_engine.unsaved_changes = false; // So it can actually close
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Save").clicked() {
                            self.quit_when_saving_done = true;
                            self.saving_progress = Some(0.0);
                        }
                    });
                });
            }
            ExportChange |ec_data, ec_mutex| {
                Modal::new(Id::new("export_changes_modal"))
                .show(ctx, |ui| {
                    ui.set_width(200.0);
                    ui.heading("Save Changes?");
                    ui.label("You have unsaved changes, do you want to save before export?");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            *ec_mutex = None;
                        }
                        if ui.button("Continue").clicked() {
                            self.exporting_progress = Some(0.0);
                            *ec_mutex = None;
                        }
                        if ui.button("Save and Continue").clicked() {
                            self.export_when_saving_done = true;
                            self.saving_progress = Some(0.0);
                            *ec_mutex = None;
                        }
                    });
                });
            }
            Exporting |exporting_data, exporting_mutex| {
                Modal::new(Id::new("exporting_modal")).show(ctx, |ui| {
                    let current_progress = exporting_data.progress;
                    ui.set_width(200.0);
                    ui.heading("Exporting ROM...");
                    ui.label("This may take time, please wait");
                    *exporting_mutex = Some(ExportingData { progress: current_progress });
                    ProgressBar::new(current_progress).ui(ui);
                    ctx.request_repaint();
                    if current_progress == 0.4 {
                        // Do the actaul export here
                        self.export_rom_file(self.exporting_to.clone());
                    }
                    if current_progress >= 1.0 {
                        *exporting_mutex = None;
                    }
                });
            }
        }
    }
}
