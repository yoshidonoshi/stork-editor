#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "2048"]

use std::sync::{LazyLock, Mutex};

use clap::Parser;
use egui::Vec2;
use gui::{gui::Gui, windows::saved_brushes::load_stored_brushes};
use log::LevelFilter;
use utils::{log_write, LogLevel};

mod utils;
mod engine;
mod data;
mod gui;

const ICON_BYTES: &[u8;486] = include_bytes!("../assets/icon.png");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short,long)]
    debug: bool
}

static CLI_ARGS: LazyLock<Args> = LazyLock::new(|| Args::parse());
static NON_MAIN_FOCUSED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

fn main() -> eframe::Result {
    let _ = simple_logging::log_to_file("stork.log", LevelFilter::Info);
    log_panics::init(); // We want it to go in stork.log

    log_write(format!("== Starting Stork Editor {} ==", VERSION), LogLevel::Log);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(1000.0, 800.0))
            // https://github.com/emilk/eframe_template/blob/50ce36a17201b32269bcd829bade159f923ef2aa/src/main.rs#L15
            .with_icon(eframe::icon_data::from_png_bytes(&ICON_BYTES[..]).unwrap())
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Stork Editor",
        options,
        Box::new(|cc| {
            // For future icons
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // Pre-ROM-load setup
            let mut gui = Box::<Gui>::default();
            if cc.egui_ctx.system_theme().is_none() {
                log_write("No default system theme found, defaulting to Dark", LogLevel::Warn);
                cc.egui_ctx.set_theme(egui::Theme::Dark);
            }
            initial_load(&mut gui);

            Ok(gui)
        })
    )
}

fn initial_load(gui: &mut Gui) {
    if let Err(error) = gui.load_sprite_csv() {
        // The software simply won't work without this. It shouldn't be possible
        log_write(format!("Sprite database load error: '{error}'"), LogLevel::Fatal);
    } else {
        log_write("Sprite database loaded successfully", LogLevel::Log);
    }
    load_stored_brushes();
    gui.display_engine.load_saved_brushes();
}
