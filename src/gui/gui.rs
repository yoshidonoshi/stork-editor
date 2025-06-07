use std::{collections::HashMap, error::Error, fmt, fs::{self, DirEntry, File}, io::Write, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

use egui::{util::undoer::Undoer, Align, ColorImage, Hyperlink, Id, Key, KeyboardShortcut, Modal, Modifiers, Pos2, ProgressBar, Rect, ScrollArea, TextureHandle, Vec2, Widget};
use rfd::FileDialog;
use strum::{EnumIter, IntoEnumIterator};
use uuid::Uuid;

use crate::{data::{mapfile::MapData, sprites::SpriteMetadata, types::{wipe_tile_cache, CurrentLayer, MapTileRecordData, Palette, BgValue}}, engine::{displayengine::{get_gameversion_prettyname, BgClipboardSelectedTile, DisplayEngine, DisplayEngineError, GameVersion}, filesys::{self, RomExtractError}}, utils::{self, color_image_from_pal, generate_bg_tile_cache, get_backup_folder, get_template_folder, get_x_pos_of_map_index, get_y_pos_of_map_index, log_write, bytes_to_hex_string, xy_to_index, LogLevel}, NON_MAIN_FOCUSED};

use super::{maingrid::render_primary_grid, sidepanel::side_panel_show, spritepanel::sprite_panel_show, toppanel::top_panel_show, windows::{brushes::show_brushes_window, col_win::collision_tiles_window, course_win::show_course_settings_window, map_segs::show_map_segments_window, palettewin::palette_window_show, paths_win::show_paths_window, resize::{show_resize_modal, ResizeSettings}, saved_brushes::show_saved_brushes_window, scen_segs::show_scen_segments_window, settings::stork_settings_window, sprite_add::sprite_add_window_show, tileswin::tiles_window_show, triggers::show_triggers_window}};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone,Copy,PartialEq,Eq,EnumIter)]
pub enum StorkTheme {
    Dark,
    Light,
    Auto
}
impl fmt::Display for StorkTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            StorkTheme::Dark => "Dark",
            StorkTheme::Light => "Light",
            StorkTheme::Auto => "System",
        };
        write!(f,"{}",text)
    }
}

/// Controls selection on BG tiles
pub struct BgSelectData {
    pub dragging: bool,
    pub start_pos: Pos2,
    pub end_pos: Pos2,
    /// Simultaneously used for drawing and retrieving of overlapped tiles
    pub selecting_rect: Rect,
    /// Indexes on the map, no other info. Careful with sizing!
    pub selected_map_indexes: Vec<u32>,
    /// Primarily to assist with converting to clipboard selections
    pub selection_width: u16,
    /// Primarily for checking selections
    pub selection_height: u16
}

impl Default for BgSelectData {
    fn default() -> Self {
        Self {
            dragging: false, start_pos: Pos2::new(0.0, 0.0), end_pos: Pos2::new(50.0, 50.0),
            selecting_rect: Rect::NOTHING, selected_map_indexes: Vec::new(),
            selection_width: 0, selection_height: 0
        }
    }
}

impl BgSelectData {
    pub fn get_selection_width(&self, map_width: u16) -> u16 {
        if self.selected_map_indexes.is_empty() {
            return 0;
        }
        let mut max_x: u16 = 0;
        let mut min_x: u16 = 0xffff;
        for map_index in &self.selected_map_indexes {
            let x_pos = get_x_pos_of_map_index(*map_index, &(map_width as u32));
            if x_pos > max_x {
                max_x = x_pos;
            }
            if x_pos < min_x {
                min_x = x_pos;
            }
        }
        if min_x > max_x {
            log_write(format!("min_x > max_x: 0x{:X} > 0x{:X}",min_x,max_x), LogLevel::Error);
            return 0;
        }
        //println!("max - min: 0x{:X} - 0x{:X} + 1 = 0x{:X}",max_x,min_x,max_x - min_x + 1);
        max_x - min_x + 1 // Because same = 0, but that's 1x1
    }

    pub fn get_selection_height(&self, map_width: u16) -> u16 {
        if self.selected_map_indexes.is_empty() {
            return 0;
        }
        let mut max_y: u16 = 0;
        let mut min_y: u16 = 0xffff;
        for map_index in &self.selected_map_indexes {
            let y_pos = get_y_pos_of_map_index(*map_index, &(map_width as u32));
            if y_pos > max_y {
                max_y = y_pos;
            }
            if y_pos < min_y {
                min_y = y_pos;
            }
        }
        if min_y > max_y {
            log_write(format!("min_y > max_y: 0x{:X} > 0x{:X}",min_y,max_y), LogLevel::Error);
            return 0;
        }
        max_y - min_y + 1 // Because same = 0, but that's 1x1
    }

    pub fn get_top_left(&mut self, map_width: u16) -> Option<Pos2> {
        if self.selected_map_indexes.is_empty() {
            return Option::None;
        }
        // Should be sorted anyway
        self.selected_map_indexes.sort();
        let x = get_x_pos_of_map_index(self.selected_map_indexes[0], &(map_width as u32));
        let y = get_y_pos_of_map_index(self.selected_map_indexes[0], &(map_width as u32));
        Some(Pos2::new(x as f32, y as f32))
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_clipboard_tiles(&mut self, map_width: u16, map_tiles: &[MapTileRecordData]) -> Vec<BgClipboardSelectedTile> {
        let mut ret: Vec<BgClipboardSelectedTile> = Vec::new();
        if self.selected_map_indexes.is_empty() {
            log_write("Attempted to convert to clipboard tiles while empty", LogLevel::Warn);
            return ret;
        }
        let Some(top_left) = self.get_top_left(map_width) else {
            log_write("Could not get top left", LogLevel::Error);
            return Vec::new();
        };
        let top_abs_x = top_left.x as i32;
        let top_abs_y = top_left.y as i32;
        for selected_map_index in &self.selected_map_indexes {
            let tile_abs_x = get_x_pos_of_map_index(*selected_map_index, &(map_width as u32)) as i32;
            let tile_abs_y = get_y_pos_of_map_index(*selected_map_index, &(map_width as u32)) as i32;
            let rel_x = tile_abs_x - top_abs_x;
            let rel_y = tile_abs_y - top_abs_y;
            let clip = BgClipboardSelectedTile {
                tile: map_tiles[*selected_map_index as usize],
                x_offset: rel_x,
                y_offset: rel_y
            };
            ret.push(clip);
        }
        ret
    }

    pub fn clear(&mut self) {
        self.dragging = false;
        self.end_pos = Pos2::ZERO;
        self.start_pos = Pos2::ZERO;
        self.selected_map_indexes.clear();
        self.selecting_rect = Rect::NOTHING;
        self.selection_height = 0;
        self.selection_width = 0;
    }
}

pub struct Gui {
    // Window states
    pub palette_window_open: bool,
    pub tile_preview_window_open: bool,
    pub brush_window_open: bool,
    pub stamps_window_open: bool,
    pub collision_window_open: bool,
    pub path_window_open: bool,
    pub sprites_window_open: bool,
    pub course_window_open: bool,
    pub area_window_open: bool,
    pub mpdz_window_open: bool,
    pub scen_window_open: bool,
    // Modals
    pub exit_changes_open: bool,
    pub saving_progress: Option<f32>,
    pub quit_when_saving_done: bool,
    pub exporting_progress: Option<f32>,
    pub exporting_to: String,
    pub export_changes_open: bool,
    pub export_when_saving_done: bool,
    pub change_course_open: bool,
    pub general_alert_popup: Option<String>,
    pub change_level_world_index: u32,
    pub change_level_level_index: u32,
    pub change_course_unsaved_changes_show: bool,
    pub change_map_unsaved_changes_show: bool,
    pub change_map_open: bool,
    pub map_change_selected_map: String,
    pub cur_level: u32,
    pub cur_world: u32,
    pub about_modal_open: bool,
    pub bug_report_modal_open: bool,
    pub clear_modal_open: bool,
    pub help_modal_open: bool,
    /// This should be stored in Gui
    pub display_engine: DisplayEngine,
    pub project_open: bool,
    pub export_directory: PathBuf, // Not yet fully mutable
    pub resize_settings: ResizeSettings,
    pub settings_open: bool,
    // Tile preview caching
    pub needs_bg_tile_refresh: bool,
    pub tile_preview_pal: usize,
    pub bg1_tile_preview_cache: Vec<TextureHandle>,
    pub bg2_tile_preview_cache: Vec<TextureHandle>,
    pub bg3_tile_preview_cache: Vec<TextureHandle>,
    pub selected_tile_preview_bg: BgValue,
    pub sprite_metadata: HashMap<u16,SpriteMetadata>,
    // Tools
    pub undoer: Undoer<MapData>,
    pub scroll_to: Option<Pos2>
}
impl Default for Gui {
    fn default() -> Self {
        Self { 
            palette_window_open: false,
            tile_preview_window_open: false,
            brush_window_open: false,
            stamps_window_open: false,
            collision_window_open: false,
            path_window_open: false,
            sprites_window_open: false,
            course_window_open: false,
            area_window_open: false,
            mpdz_window_open: false,
            scen_window_open: false,
            project_open: false,
            export_directory: PathBuf::new(), // Not yet fully mutable
            resize_settings: ResizeSettings::default(),
            settings_open: false,
            display_engine: DisplayEngine::default(),
            needs_bg_tile_refresh: false,
            tile_preview_pal: 0,
            bg1_tile_preview_cache: Vec::new(),
            bg2_tile_preview_cache: Vec::new(),
            bg3_tile_preview_cache: Vec::new(),
            selected_tile_preview_bg: BgValue::BG2, // 1-1's main ground is this
            sprite_metadata: HashMap::new(),
            exit_changes_open: false,
            saving_progress: Option::None,
            quit_when_saving_done: false,
            exporting_progress: Option::None,
            exporting_to: String::from("ERROR"),
            export_changes_open: false,
            export_when_saving_done: false,
            change_course_open: false,
            general_alert_popup: Option::None,
            change_level_world_index: 0,
            change_level_level_index: 0,
            cur_level: 0,
            cur_world: 0,
            change_course_unsaved_changes_show: false,
            change_map_unsaved_changes_show: false,
            change_map_open: false,
            map_change_selected_map: String::from(""),
            about_modal_open: false,
            bug_report_modal_open: false,
            clear_modal_open: false,
            help_modal_open: false,
            undoer: Undoer::default(),
            scroll_to: Option::None
        }
    }
}

impl Gui {
    pub fn exit(&self,ctx: &egui::Context) {
        log_write("Quitting Stork Editor".to_owned(), LogLevel::Log);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    pub fn do_open_project(&mut self) {
        if let Some(path) = FileDialog::new().pick_folder() {
            if fs::exists(&path).expect("Able to check for project path") {
                self.open_project(path);
            } else {
                log_write(format!("Project file failed existence check: '{}'",&path.display()), LogLevel::Error);
            }
        } else {
            log_write("Did not get folder path", LogLevel::Warn);
        }
    }
    pub fn do_alert(&mut self, alert_text: &str) {
        log_write(format!("Launching alert window with message '{}'",alert_text), LogLevel::Debug);
        self.general_alert_popup = Some(alert_text.to_string());
    }
    fn open_project(&mut self, path: PathBuf) {
        log_write(format!("Opening Project at '{}'",path.display()), LogLevel::Log);
        self.export_directory = path.clone();
        // Handle extracted contents
        let de: Result<DisplayEngine, DisplayEngineError> = DisplayEngine::new(path.clone());
        match de {
            Ok(de) => {
                let saved_brushes = std::mem::take(&mut self.display_engine.saved_brushes);
                self.display_engine = de; // Move it on in!
                self.display_engine.saved_brushes = saved_brushes;
            }
            Err(e) => {
                self.do_alert(&e.cause);
                return;
            }
        }
        
        let game_version = self.display_engine.game_version;
        if game_version != GameVersion::USA10 {
            let game_version_pretty = get_gameversion_prettyname(&game_version);
            let unsupported_alert = format!("You are using an unsupported version '{game_version_pretty}', saves will likely break. Supported versions: USA 1.0");
            self.do_alert(&unsupported_alert);
        }
        self.display_engine.export_folder = self.export_directory.clone();
        // Pre-load some common files
        self.display_engine.update_sprite_metadata(&self.sprite_metadata);
        self.display_engine.get_render_archive("objset.arcz");
        // Load the first level
        // 1 0 3 for BRAK and BLKZ
        // 1 4 0 for SCRL
        self.cur_world = 0;
        self.cur_level = 0;
        let cur_map_index = 0;
        match self.display_engine.load_level(self.cur_world, self.cur_level, cur_map_index) {
            Ok(_) => { /* Do nothing, it worked */},
            Err(e) => {
                // TODO: If the first map file of the project is deleted,
                //   this will soft lock, and they can never open their project...
                //   Fix this, as rare is at may be
                self.do_alert(&e);
                // It will have reverted, refresh
                self.display_engine.graphics_update_needed = true;
                return;
            }
        }
        self.needs_bg_tile_refresh = true;
        self.project_open = true;
    }
    pub fn export_rom_file(&mut self, path: String) {
        log_write(format!("Exporting ROM to '{}'",path), LogLevel::Log);
        let generate_result = filesys::generate_rom(
            &format!("{}/config.yaml",&self.export_directory.display()), &path);
        if generate_result.is_err() {
            log_write("Failed to generate ROM", LogLevel::Error);
        }
    }
    pub fn do_save(&mut self) {
        self.saving_progress = Some(0.0);
    }
    pub fn do_undo(&mut self) {
        if let Some(map_state) = self.undoer.undo(&self.display_engine.loaded_map) {
            log_write("Undoing", LogLevel::Debug);
            self.display_engine.loaded_map = map_state.clone();
            self.display_engine.unsaved_changes = true; // In case you saved
            self.display_engine.graphics_update_needed = true;
        }
    }
    pub fn do_redo(&mut self) {
        if let Some(map_state) = self.undoer.redo(&self.display_engine.loaded_map) {
            log_write("Redoing", LogLevel::Debug);
            self.display_engine.loaded_map = map_state.clone();
            self.display_engine.unsaved_changes = true; // In case you saved
            self.display_engine.graphics_update_needed = true;
        }
    }
    pub fn do_export(&mut self) {
        if self.display_engine.unsaved_changes {
            self.export_changes_open = true;
        } else {
            if let Some(path) = FileDialog::new().set_title("Export NDS ROM").set_file_name("rom.nds").save_file() {
                self.exporting_to = path.display().to_string();
                self.exporting_progress = Some(0.0);
            }
        }
    }
    pub fn do_change_course(&mut self) {
        if self.display_engine.unsaved_changes {
            self.change_course_unsaved_changes_show = true;
        } else {
            self.change_course_open = true;
        }
    }
    pub fn change_level(&mut self, world_index: u32, level_index: u32) {
        log_write("Changing Level", LogLevel::Log);
        if world_index > 5 {
            log_write(format!("Attempted to load world greater than 5: {}",world_index+1), LogLevel::Error);
            return;
        }
        if level_index > 10 {
            log_write(format!("Attempted to load level greater than 10: {}",level_index+1), LogLevel::Error);
            return;
        }
        self.clear_map_data();
        match self.display_engine.load_level(world_index, level_index,0) {
            Ok(_) => { /* Do nothing, it worked */},
            Err(e) => {
                self.do_alert(&e);
                // It will have reverted, refresh
                self.display_engine.graphics_update_needed = true;
                return;
            }
        }
        self.cur_level = level_index;
        self.cur_world = world_index;
        self.needs_bg_tile_refresh = true;
        if !self.display_engine.loaded_map.unhandled_headers.is_empty() {
            let segments_str = self.display_engine.loaded_map.unhandled_headers.join(", ");
            self.do_alert(&format!("Found unhandled map segments {}. Do not save!",segments_str));
        }
    }
    pub fn clear_map_data(&mut self) {
        wipe_tile_cache(&mut self.display_engine.tile_cache_bg1);
        self.bg1_tile_preview_cache.clear();
        wipe_tile_cache(&mut self.display_engine.tile_cache_bg2);
        self.bg2_tile_preview_cache.clear();
        wipe_tile_cache(&mut self.display_engine.tile_cache_bg3);
        self.bg3_tile_preview_cache.clear();
        self.display_engine.bg_layer_1 = Option::None;
        self.display_engine.bg_layer_2 = Option::None;
        self.display_engine.bg_layer_3 = Option::None;
        self.display_engine.bg_palettes = [Palette::default();16];
        self.display_engine.path_data = Option::None;
        self.display_engine.level_sprites.clear();
        self.display_engine.gradient_data = Option::None;
        self.display_engine.sprite_drag_status.dragging_uuid = Uuid::nil();
        self.display_engine.selected_sprite_uuids.clear();
        self.display_engine.brush_settings.cur_search_string.clear();
        self.display_engine.brush_settings.pos_brush_name.clear();
        self.display_engine.brush_settings.cur_selected_brush = Option::None;
        self.display_engine.current_brush.clear();
    }
    pub fn do_change_map(&mut self) {
        if self.display_engine.unsaved_changes {
            self.change_map_unsaved_changes_show = true;
        } else {
            self.change_map_open = true;
        }
    }
    pub fn change_map(&mut self, map_index: u32) {
        self.clear_map_data();
        match self.display_engine.load_level(self.cur_world, self.cur_level, map_index) {
            Ok(_) => { /* Do nothing, it worked */},
            Err(e) => {
                self.do_alert(&e);
                // It will have reverted, refresh
                self.display_engine.graphics_update_needed = true;
                return;
            }
        }
        self.needs_bg_tile_refresh = true;
        if !self.display_engine.loaded_map.unhandled_headers.is_empty() {
            let segments_str = self.display_engine.loaded_map.unhandled_headers.join(", ");
            self.do_alert(&format!("Found unhandled map segments {}. Do not save!",segments_str));
        }
    }
    fn save_map(&mut self) {
        log_write("Saving Map file", LogLevel::Debug);
        let file_name_ext: String = self.display_engine.loaded_map.src_file.clone();
        let _backup_res = self.backup_map();
        // Create Map file
        let file_data = self.display_engine.loaded_map.package();
        let mut file = match File::create(&file_name_ext) {
            Err(error) => {
                log_write(format!("Failed to create Map file: '{error}'"), LogLevel::Error);
                return;
            }
            Ok(f) => f,
        };
        // Write file
        match file.write_all(&file_data) {
            Err(error) => {
                log_write(format!("Failed to write Map file: '{error}'"), LogLevel::Error);
            }
            Ok(_) => {
                log_write(format!("Map file saved to '{}'",&file_name_ext), LogLevel::Log);
                self.display_engine.unsaved_changes = false;
            }
        };
    }

    fn backup_map(&mut self) -> Result<PathBuf,()> {
        log_write("Backing up current map file...", LogLevel::Debug);
        let mut backup_folder = get_backup_folder(&self.export_directory)?;
        let filename_path = Path::new(&self.display_engine.loaded_map.src_file);
        let file_name = filename_path.file_name().expect("Should be a file name for the path");
        let file_name = file_name.to_string_lossy().to_string();
        let time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time Travel").as_secs();
        backup_folder.push(format!("{}.{:?}.bak",file_name,time));
        let _copy_res = fs::copy(&self.display_engine.loaded_map.src_file, &backup_folder);
        log_write(format!("Backed up {} to {}",&self.display_engine.loaded_map.src_file,backup_folder.display()), LogLevel::Log);
        Ok(backup_folder)
    }

    fn save_course(&mut self) {
        let file_name_ext = self.display_engine.loaded_course.src_filename.clone();
        log_write(format!("Saving Course file '{}'",&file_name_ext), LogLevel::Log);
        let packed_level_file = self.display_engine.loaded_course.wrap();
        let mut file = match File::create(&file_name_ext) {
            Err(error) => {
                log_write(format!("Failed to create Course file: '{error}'"), LogLevel::Error);
                return;
            }
            Ok(f) => f,
        };
        // Write file
        if let Err(error) = file.write_all(&packed_level_file) {
            log_write(format!("Failed to write Course file: '{error}'"), LogLevel::Error);
        } else {
            log_write(format!("Course file saved to '{}'",&file_name_ext), LogLevel::Log);
            self.display_engine.unsaved_changes = false;
        }
    }
    pub fn generate_bg_cache(&self, ctx: &egui::Context, which_bg: u8, bg_pal: &Palette) -> Vec<TextureHandle> {
        puffin::profile_function!();
        let layer= match which_bg {
            0x1 => self.display_engine.bg_layer_1.as_ref(),
            0x2 => self.display_engine.bg_layer_2.as_ref(),
            0x3 => self.display_engine.bg_layer_3.as_ref(),
            _ => {
                // This should be impossible
                log_write("Invalid bg index in generate_bg_cache", LogLevel::Fatal);
                Option::None
            }
        };
        if let Some(layer_data) = &layer {
            let info = layer_data.get_info().expect("INFO exists in bg cache generator");
            if let Some(pix_tiles) = &layer_data.pixel_tiles_preview {
                let byte_count = pix_tiles.len();
                let mut byte_index: usize = 0x0;
                let mut color_imgs: Vec<ColorImage> = Vec::new();
                if info.color_mode > 0x1 {
                    log_write(format!("Color mode {} may not be well supported in bg cache generation",&info.color_mode), LogLevel::Warn);
                }
                if !info.is_256_colorpal_mode() {
                    while byte_index < byte_count {
                        let mut cur_tile_build_index: u32 = 0;
                        let mut cur_tile: Vec<u8> = Vec::new();
                        while cur_tile_build_index < 0x40 { // 64 tiles, despite being 32 bytes
                            let mut byte: u8 = 0x00;
                            if byte_index < pix_tiles.len() {
                                byte = pix_tiles[byte_index];
                            }
                            
                            byte_index += 1;

                            let lower_bits = byte % 0x10;
                            cur_tile.push(lower_bits);
                            cur_tile_build_index += 1;

                            let high_bits = byte >> 4;
                            cur_tile.push(high_bits);
                            cur_tile_build_index += 1;
                        }
                        // Pixel buffer filled, create using built-up background Palette16
                        let color_image = color_image_from_pal(bg_pal, &cur_tile);
                        color_imgs.push(color_image);
                    }
                } else {
                    if let Some(pal_256) = &layer_data.get_pltb() {
                        while byte_index < byte_count {
                            let mut cur_tile_build_index: u32 = 0;
                            let mut cur_tile: Vec<u8> = Vec::new();
                            while cur_tile_build_index < 0x40 { // 64 tiles, 1 byte each
                                let byte: u8 = pix_tiles[byte_index];
                                byte_index += 1;
                                cur_tile.push(byte);
                                cur_tile_build_index += 1;
                            }
                            // Pixel buffer filled, create using the first 256 palette attached to the background
                            let color_image = color_image_from_pal(&pal_256.palettes[0], &cur_tile);
                            color_imgs.push(color_image);
                        }
                    } else {
                        log_write(format!("generate_bg_cache: Palette not found attached to layer data in 256 bg cache update (bg layer {})",&which_bg), LogLevel::Error);
                    }
                }
                // Generate
                generate_bg_tile_cache(ctx, color_imgs)
            } else {
                log_write(format!("generate_bg_cache: Failed to retrieve pix_tiles for bg '{}'",which_bg), LogLevel::Warn);
                Vec::new()
            }
        } else {
            log_write(format!("No BG Layer found when caching layer '{}'",which_bg), LogLevel::Log);
            Vec::new()
        }
    }

    pub fn load_sprite_csv(&mut self) -> Result<(), Box<dyn Error>> {
        log_write("Loading Sprite database...".to_string(), LogLevel::Debug);
        const SPRITE_CSV: &str = include_str!("../../assets/sprites.csv");
        for line in SPRITE_CSV.lines() {
            let record: Vec<&str> = line.split(',').collect();
            let id = record[0];
            if id == "Sprite ID" {
                continue;
            }
            let id_no_prefix = id.trim_start_matches("0x");
            let true_id = match u16::from_str_radix(id_no_prefix, 16) {
                Err(error) => {
                    log_write(format!("Failure in parsing '{id_no_prefix}' as a u16: '{error}'"), LogLevel::Error);
                    continue;
                }
                Ok(id) => id,
            };
            let name = record[1];
            let description = record[2];
            let construction_function = record[3];
            let mut default_settings_len: u16 = 0xffff;
            if construction_function.starts_with("0x") {
                let setlen_no_prefix = construction_function.trim_start_matches("0x");
                match u16::from_str_radix(setlen_no_prefix, 16) {
                    Err(error) => {
                        log_write(format!("Error parsing Settings length string '{construction_function}' as hex: '{error}'"), LogLevel::Fatal);
                    }
                    Ok(func) => default_settings_len = func,
                };
            } else {
                match construction_function.parse::<u16>() {
                    Err(error) => {
                        log_write(format!("Error parsing Settings Length string '{construction_function}' as decimal: '{error}'"), LogLevel::Fatal);
                    }
                    Ok(func) => default_settings_len = func,
                };
            }
            let sprite_meta: SpriteMetadata = SpriteMetadata {
                sprite_id: true_id,
                name: name.to_string(), description: description.to_string(), default_settings_len
            };
            self.sprite_metadata.insert(true_id, sprite_meta);
        }
        Ok(())
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        puffin::profile_function!();
        if self.project_open { // Don't make loading the level an undo
            self.undoer.feed_state(ctx.input(|input| input.time), &self.display_engine.loaded_map);
        }
        let main_grid_focused = !*NON_MAIN_FOCUSED.lock().unwrap();
        // Stupid workaround for text copy crashing in input_mut
        let mut should_copy = false;
        ctx.input_mut(|i| {
            // if i.events.len() != 0 {
            //     println!("{:?}",i.events);
            // }

            if i.events.contains(&egui::Event::Copy) && main_grid_focused {
                self.do_copy();
                should_copy = true;
            }
            if i.events.contains(&egui::Event::Cut) && main_grid_focused {
                self.do_cut();
                should_copy = true;
            }
            // God DAMN this is fucking janky, why Egui why
            if i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))) && main_grid_focused {
                self.do_paste();
            }
            // Save
            if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::S)) {
                if self.project_open && self.display_engine.unsaved_changes {
                    self.do_save();
                }
            }
            // Open ROM
            if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::O)) {
                if let Err(error) = self.do_open_rom() {
                    self.do_alert(&error.cause);
                }
                return;
            }
            // Open Project
            if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::O)) {
                self.do_open_project();
                return;
            }
            // These all work normally outside of the main grid
            if main_grid_focused {
                // Undo
                if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Z)) {
                    self.do_undo();
                    return;
                }
                // Redo
                if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Y)) {
                    self.do_redo();
                    return;
                }
                // Deselect all
                if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::D)) {
                    self.do_select_all();
                    return;
                }
                // Select all
                if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::A)) {
                    self.do_select_all();
                    return;
                }
                // SPRITE CONTROLS //
                if
                    self.display_engine.display_settings.current_layer == CurrentLayer::Sprites
                    && !self.display_engine.selected_sprite_uuids.is_empty()
                {
                    let mut should_update: bool = false;
                    let mut should_deselect: bool = false;
                    for s in &self.display_engine.selected_sprite_uuids {
                        if let Ok(s) = &self.display_engine.loaded_map.get_sprite_by_uuid(*s) {
                            if i.key_pressed(egui::Key::ArrowUp) {
                                self.display_engine.loaded_map.move_sprite(s.uuid, s.x_position, s.y_position - 1);
                                should_update = true;
                                self.display_engine.unsaved_changes = true;
                            } else if i.key_pressed(egui::Key::ArrowLeft) {
                                self.display_engine.loaded_map.move_sprite(s.uuid, s.x_position - 1, s.y_position);
                                should_update = true;
                                self.display_engine.unsaved_changes = true;
                            } else if i.key_pressed(egui::Key::ArrowRight) {
                                self.display_engine.loaded_map.move_sprite(s.uuid, s.x_position + 1, s.y_position);
                                should_update = true;
                                self.display_engine.unsaved_changes = true;
                            } else if i.key_pressed(egui::Key::ArrowDown) {
                                self.display_engine.loaded_map.move_sprite(s.uuid, s.x_position, s.y_position + 1);
                                should_update = true;
                                self.display_engine.unsaved_changes = true;
                            } else if i.key_pressed(egui::Key::Delete) {
                                let _ = self.display_engine.loaded_map.delete_sprite_by_uuid(s.uuid);
                                should_deselect = true;
                                should_update = true;
                                self.display_engine.unsaved_changes = true;
                            }
                        } else {
                            log_write("Something is very wrong in handle_input, sprite_data unwrap failed", LogLevel::Error);
                        }
                    }
                    if should_update {
                        self.display_engine.graphics_update_needed = true;
                    }
                    if should_deselect {
                        self.display_engine.selected_sprite_uuids.clear();
                    }
                }
                // BG CONTROLS //
                if self.is_cur_layer_bg() {
                    if !self.display_engine.bg_sel_data.selected_map_indexes.is_empty() && !self.display_engine.bg_sel_data.dragging {
                        if i.key_pressed(egui::Key::Delete) {
                            log_write(format!("Deleting selection with {} tiles",self.display_engine.bg_sel_data.selected_map_indexes.len()), LogLevel::Log);
                            for tile_index in &self.display_engine.bg_sel_data.selected_map_indexes {
                                self.display_engine.loaded_map.delete_bg_tile_by_map_index(
                                    self.display_engine.display_settings.current_layer as u8, *tile_index);
                            }
                            self.display_engine.bg_sel_data.clear();
                            self.display_engine.graphics_update_needed = true;
                            self.display_engine.unsaved_changes = true;
                        }
                    }
                }
            }
        });

        // This crashes inside input_mut
        if should_copy {
            ctx.copy_text(String::from("StorkCopy"));
        }
    }

    fn is_cur_layer_bg(&self) -> bool {
        self.display_engine.display_settings.current_layer == CurrentLayer::BG1 ||
            self.display_engine.display_settings.current_layer == CurrentLayer::BG2 ||
            self.display_engine.display_settings.current_layer == CurrentLayer::BG3
    }

    pub fn do_open_rom(&mut self) -> Result<(),RomExtractError> {
        if let Some(path_rom) = FileDialog::new().set_title("Open YIDS ROM").set_file_name("*.nds").pick_file() {
            let display_string: String = path_rom.display().to_string();
            if display_string.contains("*") {
                // User tried to just click the load button right away
                let bad_name_msg = format!("Attempted to load file with invalid name: '{}'",&display_string);
                log_write(bad_name_msg.clone(), LogLevel::Warn);
                return Err(RomExtractError::new(&bad_name_msg));
            }
            if let Some(export_directory) = FileDialog::new().set_title("Choose folder to extract project into").pick_folder() {
                self.export_directory = export_directory;
                if !fs::exists(&self.export_directory).expect("FS Existence check should not fail") {
                    let exists_fail = "Project path failed existence check".to_string();
                    log_write(exists_fail.clone(), LogLevel::Log);
                    return Err(RomExtractError::new(&exists_fail));
                }
                if let Err(error) = filesys::extract_rom_files(&path_rom, &self.export_directory) {
                    let fail_msg = format!("Failed to extract ROM: '{error}'");
                    log_write(fail_msg.clone(), LogLevel::Error);
                    return Err(RomExtractError::new(&fail_msg));
                }
                self.open_project(self.export_directory.clone());
                self.create_map_templates();
                return Ok(());
            }
        }
        Err(RomExtractError::new("Open ROM failed"))
    }

    fn create_map_templates(&mut self) {
        log_write("Creating Map templates", LogLevel::Log);
        let map_filenames: Vec<String> = self.display_engine.course_settings.map_templates.values().cloned().collect();
        // Only one error in get_template_folder, so Option not Result
        let Some(template_dir) = get_template_folder(&self.export_directory) else {
            log_write("Failed to get or create template directory", LogLevel::Error);
            return;
        };
        let mut map_dir = self.export_directory.clone();
        map_dir.push("files");
        map_dir.push("file");
        match fs::read_dir(map_dir) {
            Ok(l) => {
                let good_dirs: Vec<DirEntry> = l.into_iter().filter_map(|x| x.ok() ).collect();
                for files_file in good_dirs {
                    let found_name = files_file.file_name().into_string().expect("NitroFS is ASCII only");
                    if map_filenames.contains(&found_name) {
                        // Found it! Copy
                        let mut to_file_dir = template_dir.clone();
                        to_file_dir.push(found_name);
                        if let Err(error) = fs::copy(files_file.path(), to_file_dir) {
                            log_write(format!("Error copying template file: '{error}'"), LogLevel::Error);
                        }
                    }
                }
            },
            Err(e) => {
                log_write(format!("Error reading map directory for templates: '{}'",e), LogLevel::Error);
            }
        }
    }

    pub fn select_sprite_from_list(&mut self, sprite_index: &usize, sprite_uuid: &Uuid) {
        log_write(format!("select_sprite_from_list: {},'{}'",sprite_index,sprite_uuid), LogLevel::Debug);
        let sprite_x_tile = self.display_engine.level_sprites[*sprite_index].x_position;
        let sprite_y_tile = self.display_engine.level_sprites[*sprite_index].y_position;
        let x_pos = sprite_x_tile as f32 * 8.0;
        let y_pos = sprite_y_tile as f32 * 8.0;
        self.scroll_to = Some(Pos2::new(x_pos, y_pos));
        self.display_engine.selected_sprite_uuids.clear();
        self.display_engine.selected_sprite_uuids.push(*sprite_uuid);
        if let Ok(spr_res) = self.display_engine.loaded_map.get_sprite_by_uuid(*sprite_uuid) {
            self.display_engine.latest_sprite_settings = bytes_to_hex_string(&spr_res.settings);
        } else {
            log_write("Failed to get sprite by UUID in select_sprite_from_list", LogLevel::Error);
        }
    }

    pub fn do_select_all(&mut self) {
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            self.display_engine.selected_sprite_uuids.clear(); // So we don't get duplicates
            for s in &self.display_engine.level_sprites {
                self.display_engine.selected_sprite_uuids.push(s.uuid);
            }
        } else if self.is_cur_layer_bg() {
            let which_bg = self.display_engine.display_settings.current_layer as u8;
            let bg_res = self.display_engine.loaded_map.get_background(which_bg);
            if let Some(bg) = bg_res {
                if let Some(tiles) = bg.get_mpbz() {
                    let all_indexes: Vec<u32> = (0..tiles.tiles.len() as u32).collect();
                    self.display_engine.bg_sel_data.selected_map_indexes = all_indexes;
                    self.display_engine.bg_sel_data.selection_width = bg.get_info().expect("Select All INFO").layer_width;
                } else {
                    log_write("MapTiles were not retrieved when seleting all", LogLevel::Error);
                }
            } else {
                log_write("BG was not retrieved when selecting all", LogLevel::Error);
            }
        }
    }

    pub fn do_select_none(&mut self) {
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            self.display_engine.selected_sprite_uuids.clear();
        } else if self.is_cur_layer_bg() {
            self.display_engine.bg_sel_data.clear();
        }
    }

    pub fn is_copy_possible(&self) -> bool {
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            !self.display_engine.selected_sprite_uuids.is_empty()
        } else if self.display_engine.display_settings.is_cur_layer_bg() {
            !self.display_engine.bg_sel_data.selected_map_indexes.is_empty()
        } else {
            false
        }
    }

    pub fn do_copy(&mut self) {
        // SPRITES
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            // Replace copied sprites
            self.display_engine.clipboard.sprite_clip.sprites.clear();
            // Find the top left one
            // This is LevelObject space, not GameFine or GUI Space
            let mut top_left_most: Pos2 = Pos2::new(999999.0, 999999.0);
            for spr_id in &self.display_engine.selected_sprite_uuids {
                let Some(lsprite) = self.display_engine.get_loaded_sprite_by_uuid(spr_id) else {
                    log_write(format!("Sprite UUID '{}' did not have an associated loaded Sprite",spr_id), LogLevel::Error);
                    continue;
                };
                let cur_sprite = lsprite.clone();
                if (cur_sprite.x_position as f32) < top_left_most.x {
                    top_left_most = Pos2::new(cur_sprite.x_position as f32, cur_sprite.y_position as f32);
                }
                if (cur_sprite.y_position as f32) < top_left_most.y {
                    top_left_most = Pos2::new(cur_sprite.x_position as f32, cur_sprite.y_position as f32);
                }
                self.display_engine.clipboard.sprite_clip.sprites.push(cur_sprite);
            }
            // Deal with found top left most
            self.display_engine.clipboard.sprite_clip.top_left_pos = top_left_most;
            // No needs for any updates, selection remains
            log_write(format!("Copied {} Sprites onto the clipboard",self.display_engine.clipboard.sprite_clip.sprites.len()), LogLevel::Log);
        } else if self.is_cur_layer_bg() {
            if self.display_engine.bg_sel_data.selected_map_indexes.is_empty() {
                log_write("Cannot copy, no BG data selected", LogLevel::Warn);
                return;
            }
            let which_bg = self.display_engine.display_settings.current_layer as u8;
            let bg_res = self.display_engine.loaded_map.get_background(which_bg);
            if let Some(bg) = bg_res {
                if let Some(tiles) = bg.get_mpbz() {
                    let clips = self.display_engine.bg_sel_data.to_clipboard_tiles(
                        bg.get_info().expect("Copy BG info guarantee").layer_width, &tiles.tiles);
                    self.display_engine.clipboard.bg_clip.tiles = clips;
                    log_write(format!("Copied {} MapTiles to clipboard",
                        self.display_engine.clipboard.bg_clip.tiles.len()
                    ), LogLevel::Log);
                } else {
                    log_write("MapTiles not retrieved when attempting to copy", LogLevel::Error);
                }
            } else {
                log_write("BG not retrieved when attempting to copy", LogLevel::Error);
            }
        } else {
            log_write("Copy not yet implemented for this layer", LogLevel::Warn);
        }
    }

    pub fn is_cut_possible(&self) -> bool {
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            !self.display_engine.selected_sprite_uuids.is_empty()
        } else if self.display_engine.display_settings.is_cur_layer_bg() {
            !self.display_engine.bg_sel_data.selected_map_indexes.is_empty()
        } else {
            false
        }
    }

    pub fn do_cut(&mut self) {
        // SPRITES
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            self.display_engine.clipboard.sprite_clip.sprites.clear();
            // Copy so we can delete properly
            let uuids_copy = self.display_engine.selected_sprite_uuids.clone();
            // Find the top left one
            // This is LevelObject space, not GameFine or GUI Space
            let mut top_left_most: Pos2 = Pos2::new(999999.0, 999999.0);
            for spr_id in &uuids_copy {
                let Some(lsprite) = self.display_engine.get_loaded_sprite_by_uuid(spr_id) else {
                    log_write(format!("Sprite UUID '{}' did not have an associated loaded Sprite",spr_id), LogLevel::Error);
                    continue;
                };
                let cur_sprite = lsprite.clone();
                if (cur_sprite.x_position as f32) < top_left_most.x {
                    top_left_most = Pos2::new(cur_sprite.x_position as f32, cur_sprite.y_position as f32);
                }
                if (cur_sprite.y_position as f32) < top_left_most.y {
                    top_left_most = Pos2::new(cur_sprite.x_position as f32, cur_sprite.y_position as f32);
                }
                self.display_engine.clipboard.sprite_clip.sprites.push(cur_sprite);
                if self.display_engine.loaded_map.delete_sprite_by_uuid(*spr_id).is_err() {
                    log_write("Failed to delete Sprite by UUID in do_cut", LogLevel::Error);
                }
            }
            // Deal with found top left most
            self.display_engine.clipboard.sprite_clip.top_left_pos = top_left_most;
            // The selection should no longer exist
            self.display_engine.selected_sprite_uuids.clear();
            self.display_engine.graphics_update_needed = true;
            self.display_engine.unsaved_changes = true;
            log_write(format!("Cut {} Sprites onto the clipboard",self.display_engine.clipboard.sprite_clip.sprites.len()), LogLevel::Log);
        }
        // TODO: check if this requires `else if` 
        if self.is_cur_layer_bg() {
            if self.display_engine.bg_sel_data.selected_map_indexes.is_empty() {
                log_write("Cannot cut, no BG data selected", LogLevel::Warn);
                return;
            }
            let which_bg = self.display_engine.display_settings.current_layer as u8;
            let bg_res = self.display_engine.loaded_map.get_background(which_bg);
            if let Some(bg) = bg_res {
                let width = bg.get_info().expect("Guaranteed INFO in BG").layer_width;
                if let Some(tiles) = bg.get_mpbz_mut() {
                    let clips = self.display_engine.bg_sel_data.to_clipboard_tiles(width, &tiles.tiles);
                    self.display_engine.clipboard.bg_clip.tiles = clips;
                    // Delete tiles that were selected
                    for tile_index in &self.display_engine.bg_sel_data.selected_map_indexes {
                        self.display_engine.loaded_map.delete_bg_tile_by_map_index(
                            self.display_engine.display_settings.current_layer as u8, *tile_index);
                    }
                    self.display_engine.bg_sel_data.clear();
                    self.display_engine.unsaved_changes = true;
                    self.display_engine.graphics_update_needed = true;
                } else {
                    log_write("MapTiles not retrieved when attempting to cut", LogLevel::Error);
                }
            } else {
                log_write("BG not retrieved when attempting to cut", LogLevel::Error);
            }
        } else {
            log_write("Cut not yet implemented for this layer", LogLevel::Warn);
        }
        
    }

    pub fn is_paste_possible(&self) -> bool {
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            !self.display_engine.clipboard.sprite_clip.sprites.is_empty()
        } else if self.is_cur_layer_bg() {
            !self.display_engine.clipboard.bg_clip.tiles.is_empty()
        } else {
            false
        }
    }

    pub fn do_paste(&mut self) {
        if !self.project_open {
            log_write("Cannot paste while project is closed", LogLevel::Log);
            return;
        }
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            log_write(format!("Pasting {} Sprites",self.display_engine.clipboard.sprite_clip.sprites.len()),LogLevel::Log);
            let tl_x = self.display_engine.clipboard.sprite_clip.top_left_pos.x as i32;
            let tl_y = self.display_engine.clipboard.sprite_clip.top_left_pos.y as i32;
            let cursor_level_x = self.display_engine.latest_square_pos_level_space.x as i32;
            let cursor_level_y = self.display_engine.latest_square_pos_level_space.y as i32;
            for copied_sprite in &mut self.display_engine.clipboard.sprite_clip.sprites {
                let stored_x = copied_sprite.x_position as i32;
                let stored_y = copied_sprite.y_position as i32;
                let x_offset = stored_x - tl_x;
                let y_offset = stored_y - tl_y;
                let true_level_x = cursor_level_x + x_offset;
                let true_level_y = cursor_level_y + y_offset;
                copied_sprite.x_position = true_level_x as u16;
                copied_sprite.y_position = true_level_y as u16;
                copied_sprite.uuid = Uuid::new_v4();
                self.display_engine.loaded_map.add_sprite(copied_sprite);
            }
            self.display_engine.graphics_update_needed = true;
            self.display_engine.unsaved_changes = true;
        } else if self.is_cur_layer_bg() {
            if self.display_engine.clipboard.bg_clip.tiles.is_empty() {
                log_write("Could not paste tiles, clipboard empty", LogLevel::Debug);
                return;
            }
            log_write(format!("Pasting {} MapTiles",self.display_engine.clipboard.bg_clip.tiles.len()), LogLevel::Log);
            let cursor_level_x = self.display_engine.latest_square_pos_level_space.x as i32;
            let cursor_level_y = self.display_engine.latest_square_pos_level_space.y as i32;
            //let mut tile_index: u32 = 0;
            let which_bg = self.display_engine.display_settings.current_layer as u8;
            let info_ro = self.display_engine.loaded_map.get_background(which_bg)
                .expect("BG should exist").get_info().expect("Info guar.");
            let layer_width = info_ro.layer_width;
            let layer_height = info_ro.layer_height;
            for tile_data in &self.display_engine.clipboard.bg_clip.tiles {
                let true_x = cursor_level_x + tile_data.x_offset;
                if true_x >= layer_width as i32 {
                    continue;
                }
                let true_y = cursor_level_y + tile_data.y_offset;
                if true_y >= layer_height as i32 {
                    continue;
                }
                let where_to_place_in_layer = xy_to_index(true_x as u32, true_y as u32, &(layer_width as u32));
                if tile_data.tile.to_short() != 0x0000 { // Dont paste blank tiles
                    self.display_engine.loaded_map.place_bg_tile_at_map_index(
                        which_bg, where_to_place_in_layer, &tile_data.tile.to_short());
                }
            }
            self.display_engine.graphics_update_needed = true;
            self.display_engine.unsaved_changes = true;
        } else {
            log_write("Paste not yet implemented for this layer", LogLevel::Warn);
        }
    }

    fn do_clear_layer(&mut self) {
        log_write(format!("Clearing layer {:?}",&self.display_engine.display_settings.current_layer),LogLevel::Log);
        match self.display_engine.display_settings.current_layer {
            CurrentLayer::BG1 => self.clear_bg_layer(1),
            CurrentLayer::BG2 => self.clear_bg_layer(2),
            CurrentLayer::BG3 => self.clear_bg_layer(3),
            CurrentLayer::Collision => {
                let Some(colz_index) = self.display_engine.loaded_map.get_bg_with_colz() else {
                    log_write("Somehow, there is no layer with COLZ during clear", LogLevel::Error);
                    return;
                };
                let Some(bg) = self.display_engine.loaded_map.get_background(colz_index) else {
                    log_write("COLZ not found in background when clearing", LogLevel::Error);
                    return;
                };
                let Some(colz) = bg.get_colz_mut() else {
                    log_write("Failed to retrieve COLZ from BG during clear", LogLevel::Error);
                    return;
                };
                colz.col_tiles.clear();
                log_write("COLZ Layer cleared", LogLevel::Debug);
                self.display_engine.graphics_update_needed = true;
                self.display_engine.unsaved_changes = true;
            }
            _ => {
                let msg = format!("Clear Layer not yet supported for {:?}",self.display_engine.display_settings.current_layer);
                log_write(msg.clone(), LogLevel::Warn);
                self.do_alert(&msg);
            }
        }
    }

    fn clear_bg_layer(&mut self, which_bg: u8) {
        log_write(format!("Wiping BG layer {}",which_bg), LogLevel::Debug);
        let Some(bg) = self.display_engine.loaded_map.get_background(which_bg) else {
            log_write(format!("No BG {} loaded to clear",which_bg), LogLevel::Warn);
            return;
        };
        let Some(map_tiles) = bg.get_mpbz_mut() else {
            log_write(format!("No map tiles on layer {} when clearing",which_bg), LogLevel::Error);
            return;
        };
        map_tiles.tiles.clear();
        log_write(format!("Cleared map tiles for bg {}",which_bg), LogLevel::Log);
        self.display_engine.unsaved_changes = true;
        self.display_engine.graphics_update_needed = true;
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::profile_function!();

        // Windowing Title
        let mut window_title: String = "Stork Editor".to_owned();
        if self.project_open {
            window_title.push_str(format!(" - {}",self.display_engine.loaded_map.map_name).as_str());
            if self.display_engine.unsaved_changes {
                window_title.push('*');
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title));
        // X button on window pressed
        if ctx.input(|i| i.viewport().close_requested())  {
            if self.display_engine.unsaved_changes {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.exit_changes_open = true;
            } else {
                self.exit(ctx);
            }
        }
        // Keyboard input
        self.handle_input(ctx);
        *NON_MAIN_FOCUSED.lock().unwrap() = false; // Reset

        // Tile storage //
        if self.needs_bg_tile_refresh {
            log_write("Regenerating BG tile cache", LogLevel::Log);
            self.needs_bg_tile_refresh = false;
            if self.tile_preview_pal >= 16 {
                // Should be completely impossible
                log_write(format!("Tiles preview palette too high: '{}'",self.tile_preview_pal), LogLevel::Fatal);
                return;
            }
            let bg_pals: &Palette = &self.display_engine.bg_palettes[self.tile_preview_pal];
            // Layer 1
            let tex_hands_1 = self.generate_bg_cache(ctx, 1, bg_pals);
            self.bg1_tile_preview_cache.clear();
            self.bg1_tile_preview_cache = tex_hands_1;
            // Layer 2
            let tex_hands_2 = self.generate_bg_cache(ctx, 2, bg_pals);
            self.bg2_tile_preview_cache.clear();
            self.bg2_tile_preview_cache = tex_hands_2;
            // Layer 3
            let tex_hands_3 = self.generate_bg_cache(ctx, 3, bg_pals);
            self.bg3_tile_preview_cache.clear();
            self.bg3_tile_preview_cache = tex_hands_3;
        }
        if self.display_engine.graphics_update_needed {
            self.display_engine.update_graphics_from_mapdata();
            self.display_engine.graphics_update_needed = false;
        }
        // Windows //
        egui::Window::new("BG Palettes")
            .open(&mut self.palette_window_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_size(Vec2::new(260.0, 235.0));
                palette_window_show(ui,&self.display_engine);  
            });
        egui::Window::new("BG Tiles")
            .open(&mut self.tile_preview_window_open)
            .resizable(false)
            .vscroll(false)
            .show(ctx, |ui: &mut egui::Ui| {
                puffin::profile_scope!("BG Tiles");
                let radio = &mut self.selected_tile_preview_bg;
                ui.set_min_size(Vec2::new(300.0,500.0));
                egui::ComboBox::from_label("Background")
                    .selected_text(format!("{radio:?}"))
                    .show_ui(ui, |ui| {
                        for bg in BgValue::iter() {
                            ui.selectable_value(radio, bg, format!("{bg:?}"));
                        }
                    });
                let cur_palette = self.tile_preview_pal;
                egui::ComboBox::from_label("Palette")
                    .selected_text(format!("{:X}",self.tile_preview_pal))
                    .show_ui(ui, |ui| {
                        for x in 0..16 {
                            ui.selectable_value(&mut self.tile_preview_pal, x, format!("0x{:X}",x));
                        }
                    });
                if cur_palette != self.tile_preview_pal {
                    self.needs_bg_tile_refresh = true;
                }
                ui.add_space(3.0);
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .min_scrolled_height(1.0)
                    .show(ui, |ui| {
                        ui.add_space(1400.0); // Number is arbitrary, just enough to fit max tile count
                        // TODO: In the future, add custom UI spacing inside tiles_window_show to make that uneeded
                        match *radio {
                            BgValue::BG1 => {
                                tiles_window_show(ui, &self.bg1_tile_preview_cache);
                            }
                            BgValue::BG2 => {
                                tiles_window_show(ui, &self.bg2_tile_preview_cache);
                            }
                            BgValue::BG3 => {
                                tiles_window_show(ui, &self.bg3_tile_preview_cache);
                            }
                        }
                    });
            });
        egui::Window::new("Add Sprites")
            .open(&mut self.sprites_window_open)
            .resizable(false)
            .max_width(400.0)
            .show(ctx, |ui| {
                if self.project_open {
                    sprite_add_window_show(ui, &mut self.display_engine, &self.sprite_metadata);
                } else {
                    ui.label("No project open");
                }
            });
        egui::Window::new("Collision Tiles")
            .open(&mut self.collision_window_open)
            .resizable(false)
            .show(ctx,|ui| {
                collision_tiles_window(ui, &mut self.display_engine);
            });
        egui::Window::new("Stork Settings")
            .open(&mut self.settings_open)
            .resizable(false)
            .show(ctx,|ui| {
                stork_settings_window(ui, &mut self.display_engine);
            });
        egui::Window::new("BG Brush")
            .open(&mut self.brush_window_open)
            .resizable(false)
            .drag_to_scroll(false)
            .max_height(600.0)
            .show(ctx, |ui| {
                show_brushes_window(ui, &mut self.display_engine);
            });
        egui::Window::new("Saved Brushes")
            .open(&mut self.stamps_window_open)
            .resizable(false)
            .drag_to_scroll(false)
            .min_height(300.0)
            .max_height(500.0)
            .show(ctx, |ui| {
                show_saved_brushes_window(ui, &mut self.display_engine);
            });
        egui::Window::new("Course Settings")
            .open(&mut self.course_window_open)
            .min_width(300.0)
            .drag_to_scroll(false)
            .show(ctx, |ui| {
                show_course_settings_window(ui, &mut self.display_engine, self.project_open);
            });
        egui::Window::new("Triggers")
            .open(&mut self.area_window_open)
            .min_width(300.0)
            .drag_to_scroll(false)
            .show(ctx, |ui| {
                show_triggers_window(ui, &mut self.display_engine);
            });
        egui::Window::new("Paths")
            .open(&mut self.path_window_open)
            .min_width(300.0)
            .drag_to_scroll(false)
            .show(ctx, |ui| {
                show_paths_window(ui, &mut self.display_engine);
            });
        egui::Window::new("Map Segments")
            .open(&mut self.mpdz_window_open)
            .min_width(300.0)
            .drag_to_scroll(false)
            .show(ctx, |ui| {
                show_map_segments_window(ui, &mut self.display_engine);
            });
        let current_layer = self.display_engine.display_settings.current_layer;
        egui::Window::new("BG Segments")
            .open(&mut self.scen_window_open)
            .min_width(300.0)
            .drag_to_scroll(false)
            .show(ctx, |ui| {
                show_scen_segments_window(ui, &mut self.display_engine,&current_layer);
            });
        // Panels //
        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            .min_height(22.0)
            .show(ctx, |ui| {
                top_panel_show(ui,self);
            });
        egui::SidePanel::right("window_panel")
            .resizable(false)
            .default_width(120.0)
            .min_width(120.0)
            .show(ctx, |ui| {
                side_panel_show(ui, self);
            });
        if self.display_engine.display_settings.current_layer == CurrentLayer::Sprites {
            egui::SidePanel::left("sprites_panel")
                .resizable(false)
                .min_width(150.0)
                .max_width(160.0)
                .show(ctx, |ui| {
                    sprite_panel_show(ui, self);
                });
        }
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ScrollArea::both()
                    .auto_shrink([false,false])
                    .drag_to_scroll(false)
                    .show_viewport(ui, |ui,viewport_rect| {
                        if let Some(scroll_to) = self.scroll_to {
                            let real_pos = ui.min_rect().left_top() + scroll_to.to_vec2();
                            ui.scroll_to_rect(Rect::from_min_size(real_pos, Vec2::new(10.0, 10.0)), Some(Align::Center));
                            self.scroll_to = Option::None;
                        }
                        if self.project_open {
                            render_primary_grid(ui, &mut self.display_engine, &viewport_rect);
                        }
                    });
            });
        // Modals //
        if self.resize_settings.window_open {
            let _resize_modal = Modal::new(Id::new("resize_modal"))
                .show(ctx, |ui| {
                    show_resize_modal(ui, &mut self.display_engine, &mut self.resize_settings);
                });
        }
        self.general_alert_popup.take_if(|alert| {
            let alert_modal = Modal::new(Id::new("alert_modal"))
                .show(ctx, |ui| {
                    ui.set_width(200.0);
                    ui.heading("Alert");
                    ui.label(alert.as_str());
                    ui.button("Okay").clicked()
                });
            alert_modal.inner
        });
        if self.exit_changes_open {
            let _save_modal = Modal::new(Id::new("close_changes_modal"))
                .show(ctx, |ui| {
                    ui.set_width(200.0);
                    ui.heading("Save Changes?");
                    ui.label("You have unsaved changes, do you want to save before you exit?");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.exit_changes_open = false;
                        }
                        if ui.button("Discard").clicked() {
                            self.exit_changes_open = false;
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
        if self.export_changes_open {
            let _export_change_modal = Modal::new(Id::new("export_changes_modal"))
                .show(ctx, |ui| {
                    ui.set_width(200.0);
                    ui.heading("Save Changes?");
                    ui.label("You have unsaved changes, do you want to save before export?");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.export_changes_open = false;
                        }
                        if ui.button("Continue").clicked() {
                            self.exporting_progress = Some(0.0);
                            self.export_changes_open = false;
                        }
                        if ui.button("Save and Continue").clicked() {
                            self.export_when_saving_done = true;
                            self.saving_progress = Some(0.0);
                            self.export_changes_open = false;
                        }
                    });
                });
        }
        if let Some(exporting_progress) = self.exporting_progress {
            egui::Modal::new(Id::new("exporting_modal")).show(ctx, |ui| {
                ui.set_width(200.0);
                ui.heading("Exporting ROM...");
                ui.label("This may take time, please wait");
                ProgressBar::new(exporting_progress).ui(ui);
                self.exporting_progress = Some(exporting_progress + 0.1);
                ctx.request_repaint();
                if exporting_progress == 0.4 {
                    // Do the actaul export here
                    self.export_rom_file(self.exporting_to.clone());
                }
                if exporting_progress >= 1.0 {
                    self.exporting_progress = Option::None;
                }
            });
        }
        if let Some(saving_progress) = self.saving_progress {
            egui::Modal::new(Id::new("saving_modal")).show(ctx, |ui| {
                ui.set_width(70.0);
                ui.heading("Saving...");
                ProgressBar::new(saving_progress).ui(ui);
                if saving_progress == 0.0 {
                    ctx.request_repaint();
                }
                if saving_progress == 0.4 {
                    self.save_map();
                    self.save_course();
                }
                if saving_progress >= 1.0 {
                    self.saving_progress = Option::None;
                    self.display_engine.unsaved_changes = false;
                    if self.quit_when_saving_done {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if self.export_when_saving_done {
                        self.export_when_saving_done = false;
                        self.do_export();
                    }
                } else {
                    self.saving_progress = Some(saving_progress + 0.2);
                }
            });
        }
        if self.change_course_unsaved_changes_show {
            let _export_change_modal = Modal::new(Id::new("course_changes_modal"))
            .show(ctx, |ui| {
                ui.set_width(200.0);
                ui.heading("Save Changes?");
                ui.label("You have unsaved changes, do you want to save before changing Course?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.change_course_unsaved_changes_show = false;
                    }
                    if ui.button("Continue").clicked() {
                        self.change_course_unsaved_changes_show = false;
                        self.change_course_open = true;
                    }
                    if ui.button("Save and Continue").clicked() {
                        self.change_course_unsaved_changes_show = false;
                        self.change_course_open = true;
                        self.do_save();
                    }
                });
            });   
        }
        if self.change_map_unsaved_changes_show {
            let _export_change_modal = Modal::new(Id::new("map_changes_modal"))
            .show(ctx, |ui| {
                ui.set_width(200.0);
                ui.heading("Save Changes?");
                ui.label("You have unsaved changes, do you want to save before changing map?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.change_map_unsaved_changes_show = false;
                    }
                    if ui.button("Continue").clicked() {
                        self.change_map_unsaved_changes_show = false;
                        self.change_map_open = true;
                    }
                    if ui.button("Save and Continue").clicked() {
                        self.change_map_unsaved_changes_show = false;
                        self.change_map_open = true;
                        self.do_save();
                    }
                });
            });  
        }
        if self.change_map_open {
            egui::Modal::new(Id::new("map_change_modal")).show(ctx, |ui| {
                ui.heading("Select map");
                ui.set_width(150.0);

                let crsb = self.display_engine.loaded_course.level_map_data.clone();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (map_index, map) in crsb.iter().enumerate() {
                        let mut but = ui.button(&map.map_filename_noext);
                        if map.map_filename_noext == self.display_engine.loaded_map.map_name {
                            but = but.highlight();
                        }
                        if but.clicked() {
                            // Since the targeting is done via GUI, but accesses the saved data
                            self.save_course();
                            // This is to be used once support for ALL map selection is working
                            self.map_change_selected_map = map.map_filename_noext.clone();
                            self.change_map(map_index as u32);
                            self.change_map_open = false;
                        }
                    }
                });
                
                // let _map_change_selected_map = egui::ComboBox::from_label("")
                //     .selected_text(format!("{}",&self.map_change_selected_map))
                //     .show_ui(ui, |ui| {
                //         // Get the paths
                //         let read_res = fs::read_dir(nitrofs_abs(&self.display_engine.export_folder, &"".to_owned()));
                //         if read_res.is_err() {
                //             log_write(format!("Failed to read export directory: '{}'",read_res.unwrap_err()), LogLevel::disable);
                //             return;
                //         }
                //         let paths = read_res.unwrap();
                //         // Loop
                //         for path in paths {
                //             if path.is_err() {
                //                 log_write(format!("Failed to unwrap path in map change: '{}'",path.unwrap_err()), LogLevel::Error);
                //             } else {
                //                 let path = path.unwrap();
                //                 if path.path().is_dir() {
                //                     continue;
                //                 }
                //                 if path.path().extension().expect("Extension should exist").to_str().expect("its a goddamn string") == "mpdz" {
                //                     let label = path.file_name().into_string().unwrap_or(format!("ERROR"));
                //                     ui.selectable_value(&mut self.map_change_selected_map, label.clone(), &label);
                //                 }
                //             }
                //         }

                //     });

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.change_map_open = false;
                    }
                });
            });
        }
        if self.change_course_open {
            egui::Modal::new(Id::new("course_change_modal")).show(ctx, |ui| {
                ui.heading("Select a Course");
                ui.set_width(150.0);
                // World Selection //
                let _combo_world = egui::ComboBox::new(
                    egui::Id::new("change_level_world"), "World")
                    .selected_text(format!("{}",self.change_level_world_index+1))
                    .show_ui(ui, |ui| {
                        for x in 0..5_u32 {
                            ui.selectable_value(&mut self.change_level_world_index, x, (x+1).to_string());                          
                        }
                    });
                let _combo_level = egui::ComboBox::new(
                    egui::Id::new("change_level_level"), "Level")
                    .selected_text(format!("{}",self.change_level_level_index+1))
                    .show_ui(ui, |ui| {
                        for y in 0..10_u32 {
                            ui.selectable_value(&mut self.change_level_level_index, y, (y+1).to_string());
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.change_course_open = false;
                    }
                    if ui.button("Okay").clicked() {
                        self.change_course_open = false;
                        self.change_level(self.change_level_world_index, self.change_level_level_index);
                    }
                });
            });
        }
        if self.about_modal_open {
            let about_modal = Modal::new(egui::Id::new("about_modal"));
            about_modal.show(ctx, |ui| {
                ui.heading(format!("Stork {}",VERSION));
                ui.label("A ROM-hacking tool for Yoshi's Island DS");
                ui.label("Created by YoshiDonoshi/Zolarch");
                ui.add(Hyperlink::from_label_and_url("Source Code", env!("GITHUB_REPO")));
                ui.vertical_centered(|ui| {
                    let about_close_button = ui.button("Close");
                    if about_close_button.clicked() {
                        self.about_modal_open = false;
                    }
                });
            });
        }
        if self.bug_report_modal_open {
            let bug_modal = Modal::new(egui::Id::new("bug_report_modal"));
            bug_modal.show(ctx, |ui| {
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
                        self.bug_report_modal_open = false;
                    }
                });
            });
        }
        if self.clear_modal_open {
            let clear_modal = Modal::new(egui::Id::new("clear_all_modal"));
            clear_modal.show(ctx, |ui| {
                ui.heading("Clear Layer");
                ui.label(format!("This will delete everything on the current layer ({:?})",&self.display_engine.display_settings.current_layer));
                ui.label("Are you sure?");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.clear_modal_open = false;
                    }
                    if ui.button("Clear Layer").clicked() {
                        self.do_clear_layer();
                        self.clear_modal_open = false;
                    }
                });
            });
        }
        if self.help_modal_open {
            let help_modal = Modal::new(egui::Id::new("help_modal"));
            help_modal.show(ctx, |ui| {
                ui.heading("Help");
                ui.label("First, check out the documentation and FAQ:");
                ui.hyperlink(env!("DOC_URL"));
                ui.label("If you're still having trouble, ask a question on the Discord server:");
                ui.hyperlink(env!("DISCORD"));
                ui.vertical_centered(|ui| {
                    if ui.button("Close").clicked() {
                        self.help_modal_open = false;
                    }
                });
            });
        }
        if self.display_engine.course_settings.add_window_open {
            let add_map_modal = Modal::new(egui::Id::new("add_map_modal"));
            add_map_modal.show(ctx, |ui| {
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
                        self.display_engine.course_settings.add_window_open = false;
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
                        self.display_engine.course_settings.add_window_open = false;
                        self.display_engine.unsaved_changes = true;
                        self.display_engine.graphics_update_needed = true;
                    }
                });
            });
        }
    }
}

