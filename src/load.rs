use std::{sync::LazyLock, time::Instant};

use egui::ahash::{HashMap, HashMapExt};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{data::sprites::SpriteMetadata, gui::{gui::Gui, windows::saved_brushes::load_stored_brushes}, utils::{log_write, LogLevel}};

pub static SPRITE_METADATA: LazyLock<HashMap<u16,SpriteMetadata>> = LazyLock::new(load_sprite_csv);

pub fn initial_load(gui: &mut Gui) {
    let gui_loading_time = Instant::now();
    gui.display_engine.load_saved_brushes();
    log_write(format!("Took {:#?} for the GUI load", gui_loading_time.elapsed()), LogLevel::Debug);

    let static_loading_time = Instant::now();
    [
        || load_sprite_metadata(),
        || load_stored_brushes(),
    ]
        .into_par_iter()
        .for_each(|f| f());
    log_write(format!("Took {:#?} for the STATIC load", static_loading_time.elapsed()), LogLevel::Debug);
}

const SPRITE_CSV: &str = include_str!("../assets/sprites.csv");

fn load_sprite_metadata() {
    log_write("Loading Sprite database...", LogLevel::Debug);
    LazyLock::force(&SPRITE_METADATA);
    log_write("Loaded sprite database successfully", LogLevel::Log);
}

fn load_sprite_csv() -> HashMap<u16, SpriteMetadata> {
    let mut sprite_metadata = HashMap::new(); 

    for line in SPRITE_CSV.lines().skip(1) {
        let mut iter = line.split(',');

        let [id, name, description, len, _construction_function] =
            std::array::from_fn(|_| iter.next().unwrap_or_else(|| panic!("Invalid Sprite CSV, line '{line}', doesn't contain 5 or more columns")));
        // let settings: Vec<&str> = iter.collect(); // this can get uncommented if needed

        // ID parsing
        let id_no_prefix = id.trim_start_matches("0x");
        let true_id = match u16::from_str_radix(id_no_prefix, 16) {
            Err(error) => {
                log_write(format!("Failure in parsing '{id_no_prefix}' as a u16: '{error}'"), LogLevel::Error);
                continue;
            }
            Ok(id) => id,
        };

        // LEN parsing
        let is_hex = len.starts_with("0x");
        let settings_len_base = match is_hex {
            true => 16,
            false => 10,
        };
        let default_settings_len = match u16::from_str_radix(len.trim_start_matches("0x"), settings_len_base) {
            Err(error) => {
                let kind = match is_hex {
                    true => "hex",
                    false => "decimal",
                };
                log_write(format!("Error parsing Settings length string '{len}' as {kind}: '{error}'"), LogLevel::Fatal);
                unreachable!()
            }
            Ok(func) => func,
        };
        let sprite_meta = SpriteMetadata {
            sprite_id: true_id,
            name: name.to_string(), description: description.to_string(),
            default_settings_len,
        };
        sprite_metadata.insert(true_id, sprite_meta);
    }

    sprite_metadata
}
