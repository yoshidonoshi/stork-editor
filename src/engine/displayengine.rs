// Consider this the NDS' graphical memory and settings, plus helpers

use std::{collections::HashMap, fmt::{self, Display}, fs::{self, read_to_string}, io::Cursor, path::PathBuf};

use egui::{Pos2, Rect};
use serde_yml::Value;
use uuid::Uuid;

use crate::{data::{area::TriggerSettings, backgrounddata::BackgroundData, course_file::{CourseInfo, MapExit}, grad::GradientData, mapfile::MapData, path::{PathDatabase, PathSettings}, rarc::RenderArchive, sprites::{LevelSprite, SpriteMetadata}, types::{CurrentLayer, MapTileRecordData, Palette, TileCache}, TopLevelSegment}, gui::{gui::{BgSelectData, StorkTheme}, windows::{brushes::{Brush, BrushSettings}, course_win::CourseSettings}}, utils::{self, log_write, nitrofs_abs}};

use crate::utils::LogLevel;

/// Global, not specifically tied to individual layer data
pub struct DisplaySettings {
    pub current_layer: CurrentLayer,
    pub show_bg1: bool,
    pub show_bg2: bool,
    pub show_bg3: bool,
    pub show_col: bool,
    pub show_sprites: bool,
    pub show_paths: bool,
    pub show_entrances: bool,
    pub show_exits: bool,
    pub show_breakable_rock: bool,
    pub show_triggers: bool,
    pub stork_theme: StorkTheme,
    pub show_box_for_rendered: bool
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            // Start on Sprites
            current_layer: CurrentLayer::Sprites,
            show_bg1: true,
            show_bg2: true,
            show_bg3: true,
            show_col: true,
            show_sprites: true,
            show_paths: true,
            show_entrances: true,
            show_exits: true,
            // Since it's just a copy overlay
            show_breakable_rock: false,
            show_triggers: true,
            stork_theme: StorkTheme::AUTO,
            show_box_for_rendered: true
        }
    }
}

impl DisplaySettings {
    pub fn is_cur_layer_bg(&self) -> bool {
        (self.current_layer == CurrentLayer::BG1) || (self.current_layer == CurrentLayer::BG2) || (self.current_layer == CurrentLayer::BG3)
    }
}

#[derive(PartialEq,Clone,Copy,Debug)]
pub enum GameVersion {
    /// AYWE
    USA10, // r0
    USA11, // r1
    USAXX, // Unknown revision
    /// AYWP
    EUR10, // r0
    EUR11, // r1
    EURXX, // Unknown revision
    /// AYWJ
    JAP,
    /// What?
    UNKNOWN
}
pub fn get_gameversion_prettyname(gv: &GameVersion) -> String {
    match gv {
        GameVersion::EUR10 => String::from("EUR 1.0"),
        GameVersion::EUR11 => String::from("EUR 1.1 (rev1)"),
        GameVersion::EURXX => String::from("Unknown EUR"),
        GameVersion::JAP => String::from("JPN"),
        GameVersion::USA10 => String::from("USA 1.0"),
        GameVersion::USA11 => String::from("USA 1.1 (rev1)"),
        GameVersion::USAXX => String::from("Unknown USA"),
        GameVersion::UNKNOWN => String::from("Unknown Game Version")
    }
}

#[derive(Debug)]
pub struct DisplayEngineError {
    pub cause: String
}
impl DisplayEngineError {
    pub fn new(cause: String) -> Self {
        Self {
            cause,
        }
    }
}
impl Display for DisplayEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error initializing Display Engine: '{}'", &self.cause)
    }
}

pub struct SpriteDragStatus {
    pub start_x: f32,
    pub start_y: f32,
    pub dragging_uuid: Uuid
}
impl Default for SpriteDragStatus {
    fn default() -> Self {
        Self {
            start_x: 0.0, start_y: 0.0,
            dragging_uuid: Uuid::nil()
        }
    }
}

pub struct ColDragStatus {
    pub start_pos: Pos2,
    pub end_pos: Pos2,
    pub selecting_rect: Rect,
    pub dragging: bool,
    /// Once set to true, delete everything underneath selection, then set to false
    pub delete_under: bool
}
impl Default for ColDragStatus {
    fn default() -> Self {
        Self {
            start_pos: Pos2::new(0.0, 0.0),
            end_pos: Pos2::new(0.0, 0.0),
            selecting_rect: Rect::NOTHING,
            dragging: false, delete_under: false
        }
    }
}

#[derive(Clone)]
pub struct SpriteClipboard {
    pub sprites: Vec<LevelSprite>,
    pub top_left_pos: Pos2
}
impl Default for SpriteClipboard {
    fn default() -> Self {
        Self {
            sprites: Vec::new(),
            top_left_pos: Pos2::new(0.0, 0.0)
        }
    }
}

#[derive(Clone,Copy,Debug)]
pub struct BgClipboardSelectedTile {
    pub tile: MapTileRecordData,
    pub x_offset: i32,
    pub y_offset: i32
}
impl fmt::Display for BgClipboardSelectedTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"BgClipboardSelectedTile [ Tile=0x{:X}, xOffset=0x{:X}, yOffset=0x{:X} ]",self.tile.to_short(),self.x_offset,self.y_offset)
    }
}

#[derive(Clone,Debug,Default)]
pub struct BgClipboard {
    pub tiles: Vec<BgClipboardSelectedTile>
}
impl BgClipboard {
    pub fn clear(&mut self) {
        self.tiles.clear();
    }
}

#[derive(Default)]
pub struct Clipboard {
    pub sprite_clip: SpriteClipboard,
    pub bg_clip: BgClipboard
}

/// NDS Graphical data and memory, tailored for YIDS
pub struct DisplayEngine {
    pub loaded_map: MapData,
    pub map_index: Option<usize>,
    pub loaded_course: CourseInfo,
    pub bg_palettes: [Palette;16],
    pub bg_layer_1: Option<BackgroundData>,
    pub bg_layer_2: Option<BackgroundData>,
    pub bg_layer_3: Option<BackgroundData>,
    pub tile_cache_bg1: TileCache,
    pub tile_cache_bg2: TileCache,
    pub tile_cache_bg3: TileCache,
    pub level_sprites: Vec<LevelSprite>,
    pub gradient_data: Option<GradientData>,
    pub path_data: Option<PathDatabase>,
    pub path_settings: PathSettings,
    pub loaded_archives: HashMap<String,RenderArchive>,
    pub loaded_arm9: Option<Vec<u8>>,
    pub game_version: Option<GameVersion>,
    pub display_settings: DisplaySettings,
    pub selected_sprite_uuids: Vec<Uuid>,
    pub selected_sprite_to_place: Option<u16>,
    pub col_tile_to_place: u8,
    // This does not change, and therefore can be cloned at will
    pub sprite_metadata_copy: HashMap<u16,SpriteMetadata>,
    pub latest_sprite_settings: String,
    pub sprite_search_query: String,
    pub sprite_drag_status: SpriteDragStatus,
    pub col_selector_status: ColDragStatus,
    pub unsaved_changes: bool,
    pub export_folder: PathBuf,
    pub current_brush: Brush,
    pub brush_settings: BrushSettings,
    pub saved_brushes: Vec<Brush>,
    pub graphics_update_needed: bool,
    pub clipboard: Clipboard,
    pub latest_square_pos_level_space: Pos2,
    pub course_settings: CourseSettings,
    pub trigger_settings: TriggerSettings,
    pub bg_sel_data: BgSelectData,
    pub tile_hover_pos: Pos2
}

impl Default for DisplayEngine {
    fn default() -> Self {
        Self {
            loaded_map: MapData::default(),
            map_index: Option::None,
            loaded_course: CourseInfo::default(),
            bg_palettes: Default::default(),
            bg_layer_1: Option::None, bg_layer_2: Option::None, bg_layer_3: Option::None,
            loaded_arm9: Option::None,
            game_version: Option::None,
            tile_cache_bg1: vec![vec![Option::None;1024];16],
            tile_cache_bg2: vec![vec![Option::None;1024];16],
            tile_cache_bg3: vec![vec![Option::None;1024];16],
            level_sprites: Vec::new(),
            gradient_data: Option::None,
            path_data: Option::None,
            path_settings: PathSettings::default(),
            display_settings: DisplaySettings::default(),
            loaded_archives: HashMap::new(),
            selected_sprite_uuids: Vec::new(),
            selected_sprite_to_place: Option::None,
            col_tile_to_place: 0x1, // Basic square
            sprite_metadata_copy: HashMap::new(),
            latest_sprite_settings: String::from(""),
            sprite_search_query: String::from(""),
            sprite_drag_status: SpriteDragStatus::default(),
            col_selector_status: ColDragStatus::default(),
            unsaved_changes: false,
            export_folder: PathBuf::new(),
            current_brush: Brush::default(),
            brush_settings: BrushSettings::default(),
            saved_brushes: Vec::new(),
            graphics_update_needed: false,
            clipboard: Clipboard::default(),
            latest_square_pos_level_space: Pos2::new(0.0, 0.0),
            course_settings: CourseSettings::default(),
            trigger_settings: TriggerSettings::default(),
            bg_sel_data: BgSelectData::default(),
            tile_hover_pos: Pos2::ZERO
        }
    }
}

impl DisplayEngine {
    pub fn new(extract_dir: PathBuf) -> Result<DisplayEngine, DisplayEngineError> {
        let mut de = DisplayEngine::default(); // Everything is empty

        // Build Stamp //
        let rc_path: PathBuf = PathBuf::from(&extract_dir);
        let stamp_rc_path = nitrofs_abs(&rc_path, &"stamp.rc".to_owned());
        let build_date = match read_to_string(stamp_rc_path) {
            Err(error) => {
                let rc_err1 = format!("Failed to open stamp.rc: {error}");
                log_write(rc_err1.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(rc_err1));
            }
            Ok(d) => d,
        };

        // Check Header //
        let mut header_path: PathBuf = PathBuf::from(&extract_dir);
        header_path.push("header.yaml");
        let yaml_content = match read_to_string(header_path) {
            Err(error) => {
                let yaml_err1 = format!("Failed to open header.yaml: {error}");
                log_write(yaml_err1.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(yaml_err1));
            }
            Ok(s) => s,
        };
        let yaml: Value = serde_yml::from_str(&yaml_content).expect("Failed to parse header.yaml");
        if let Some(game_code) = yaml["gamecode"].as_str() {
            // Does not get the revision, do that later
            let game_ver = match game_code {
                "AYWE"=> GameVersion::USAXX,
                "AYWP"=> GameVersion::EURXX,
                "AYWJ"=> GameVersion::JAP, // Only one Japanese version
                _=> GameVersion::UNKNOWN
            };
            log_write(format!("Found game version header: '{}'",game_code), LogLevel::DEBUG);
            de.game_version = Some(game_ver);
        }
        if let Some(maker_code) = yaml["makercode"].as_str() {
            if maker_code == "01" {
                log_write("Game is unmodified".to_owned(), LogLevel::LOG);
            } else if maker_code == "63" {
                log_write("Game was edited with Stork".to_owned(), LogLevel::LOG);
            } else {
                log_write(format!("Unusual makercode: '{}'",maker_code), LogLevel::WARN);
            }
        }

        // Open and check ARM9 Binary //
        let mut arm9_path: PathBuf = PathBuf::from(&extract_dir);
        arm9_path.push("arm9");
        arm9_path.push("arm9.bin");
        if let None|Some(false) = fs::exists(&arm9_path).ok() {
            let arm9_inval_path = format!("ARM9 Path invalid: '{}'",&arm9_path.display());
            log_write(arm9_inval_path.clone(), LogLevel::ERROR);
            return Err(DisplayEngineError::new(arm9_inval_path));
        }
        let contents = match fs::read(&arm9_path) {
            Ok(bytes) => {
                log_write(format!("Loaded ARM9 binary from '{}' successfully",&arm9_path.display()), LogLevel::LOG);
                bytes
            }
            Err(e) => {
                let arm9_io_err = format!("ARM9 IO error: {}", e);
                log_write(arm9_io_err.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(arm9_io_err));
            }
        };
        de.loaded_arm9 = Some(contents);

        // Get Revision
        let gamever = de.game_version.expect("Gameversion set"); // Copies
        match gamever {
            GameVersion::USAXX => {
                de.game_version = Some(match build_date.as_str() {
                    "061110.1620" => GameVersion::USA11,
                    "061009.0352" => GameVersion::USA10,
                    _ => GameVersion::USAXX
                });
            }
            GameVersion::EURXX => {
                de.game_version = Some(match build_date.as_str() {
                    "061009.0352" => GameVersion::EUR10,
                    "061110.1620" => GameVersion::EUR11,
                    _ => GameVersion::EURXX
                });
            }
            GameVersion::UNKNOWN => {
                //let _ = fs::remove_dir_all(extract_dir).expect("Should remove directory on unknown game");
                let unsupported_msg = "Game Version is unknown, canceling load".to_owned();
                log_write(unsupported_msg.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(unsupported_msg));
            }
            GameVersion::JAP => {
                let jpn_msg = "JPN version not yet supported, will break".to_owned();
                log_write(jpn_msg.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(jpn_msg))
            }
            _ => {
                let bad_logic_gamever = format!("Game version {:?} should not be hit here",gamever);
                //let _ = fs::remove_dir_all(extract_dir).expect("Should remove directory on unsupported game");
                log_write(bad_logic_gamever.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(bad_logic_gamever));
            }
        }

        // Version checks //
        let got_contents = de.loaded_arm9.as_ref().expect("ARM9 was loaded properly");
        let game_version = de.game_version.expect("Must be a version");
        match game_version {
            GameVersion::USA10 => {
                let found_str = utils::read_fixed_string(got_contents, 0xe1e6e, 6);
                if !found_str.eq("1-1_D3") {
                    let unk_ver1 = "Could not find 1-1_D3 in USA 1.0".to_string();
                    log_write(unk_ver1.clone(), LogLevel::ERROR);
                    return Err(DisplayEngineError::new(unk_ver1));
                }
            },
            GameVersion::USA11 => {
                let found_str2 = utils::read_fixed_string(got_contents, 0x0e20ae, 6);
                if !found_str2.eq("1-1_D3") {
                    let unk_ver2 = "Could not find 1-1_D3 in USA 1.1".to_string();
                    log_write(unk_ver2.clone(), LogLevel::ERROR);
                    return Err(DisplayEngineError::new(unk_ver2));
                }
                log_write("USA 1.1 is poorly supported, proceed with caution", LogLevel::WARN);
            }
            GameVersion::USAXX => {
                let unk_ver3 = "Unknown USA version".to_string();
                log_write(unk_ver3.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(unk_ver3));
            }
            GameVersion::EURXX => {
                let unk_ver3 = "Unknown EUR version".to_string();
                log_write(unk_ver3.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(unk_ver3));
            }
            GameVersion::EUR10 => {
                let unk_ver3 = "EURr0 unsupported".to_string();
                log_write(unk_ver3.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(unk_ver3));
            }
            GameVersion::EUR11 => {
                let unk_ver3 = "EURr1 unsupported".to_string();
                log_write(unk_ver3.clone(), LogLevel::ERROR);
                return Err(DisplayEngineError::new(unk_ver3));
            }
            _ => {
                log_write("This should be impossible to hit in version test", LogLevel::FATAL);
            }
        }
        log_write(format!("Assuming game version {}",get_gameversion_prettyname(&game_version)), LogLevel::LOG);
        Ok(de)
    }

    fn get_level_filename(&self, world_index: &u32, level_index: &u32) -> String {
        let Some(game_ver) = self.game_version else {
            // Should be impossible
            log_write("Attempted to call get_level_filename before game opened", LogLevel::FATAL);
            unreachable!();
        };
        let filename_res = match game_ver {
            GameVersion::USA10 => self.get_level_filename_usa(world_index, level_index,GameVersion::USA10),
            GameVersion::USA11 => self.get_level_filename_usa(world_index, level_index,GameVersion::USA11),
            //GameVersion::EUR => self.get_level_filename_eur_11(world_index, level_index),
            _ => {
                log_write(format!("Attempted to get level filename on unsupported version: '{game_ver:?}'"), LogLevel::FATAL);
                unreachable!();
            },
        };
        match filename_res {
            Ok(s) => {
                s
            }
            Err(e) => {
                log_write(format!("filename_res failed somehow: {}",e), LogLevel::FATAL);
                "Error".to_owned()
            }
        }
    }

    /// This function found at 0x02050000 in USA 1.0. Modified as little as possible.
    /// 
    /// TODO: Make real errors
    fn get_level_filename_usa(&self, world_index: &u32, level_index: &u32, game_version: GameVersion) -> Result<String,String> {
        if world_index + 1 > 5 {
            let world_fail = "World 5 is the highest World";
            log_write(world_fail, LogLevel::ERROR);
            return Err(world_fail.to_owned());
        }

        if level_index + 1 > 10 {
            let level_fail = "There are only 10 levels per World";
            log_write(level_fail, LogLevel::ERROR);
            return Err(level_fail.to_owned());
        }

        // This +1 is due to 0-1 being at the base of the array
        // That would mean 1-1 (indexes 0-0) leads to 0-1 not 1-1
        // So the +1 makes it skip that
        let level_id: u32 = world_index * 10 + level_index + 1;
        if level_id < 0x7b || level_id > 0x7e {
            // 02050024 (some function that takes in 0), does not break
        }
        if level_id == 0 {
            return Ok("0-1_D3".to_string());
        } else {
            if level_id == 0x7a {
                // FUN_020173c0(0xd,1);
                // Enemy Check, aka Museum
                return Ok("ene_check_".to_owned());
            } else if level_id == 0x7b {
                return Ok("koopa3".to_owned());
            } else if level_id == 0x7c {
                return Ok("koopa2".to_owned());
            } else if level_id == 0x7d {
                return Ok("kuppa".to_owned());
            } else if level_id == 0x7e {
                return Ok("lastback".to_owned());
            } else if level_id == 0x7f {
                return Err("0x7f unknown multi".to_owned());
            }
        }

        if level_id > 99 {
            return Err(">99 unknown multi".to_owned());
        }
        const LEVEL_ARRAY_ADDR_USA11: u32 = 0x000d9178; // 0x020d9178
        const LEVEL_ARRAY_ADDR_USA10: u32 = 0x000d8f20; // 0x000d8e58;
        let mut level_array_addr = LEVEL_ARRAY_ADDR_USA10;
        if game_version == GameVersion::USA11 {
            level_array_addr = LEVEL_ARRAY_ADDR_USA11;
        }
        let offset = level_id * 4; // u32 = 4 bytes
        let array_internal_address = level_array_addr + offset;
        // Make this the smarter way eventually
        if let Some(arm9_binary) = &self.loaded_arm9 {
            let mut rdr: Cursor<&Vec<u8>> = Cursor::new(arm9_binary);
            rdr.set_position(array_internal_address as u64);
            let string_address: u32 = match utils::read_address(&mut rdr) {
                Some(s) => s,
                None => {
                    let err_msg = "Failed to get string address in level name retrieval".to_owned();
                    log_write(err_msg.clone(), LogLevel::ERROR);
                    return Err(err_msg)
                },
            };
            rdr.set_position(string_address as u64);
            let level_name = utils::read_c_string(&mut rdr);
            Ok(level_name)
        } else {
            Err("NO BINARY".to_owned())
        }
    }

    #[allow(dead_code)]
    fn get_level_filename_eur_11(&self, world_index: &u32, level_index: &u32) -> Result<String,String> {
        // 1-1 filename location: 0xe21ae
        let level_id: u32 = world_index * 10 + level_index;// + 1 maybe not here?
        if (level_id < 0x7b) || (0x7e < level_id) {
            //func_02017e88(0);
        }
        // if ((int)param_1 < 1) {
        //     if (param_1 == 0) {
        //     return "0-1_D3";
        //     }
        // }
        if level_id == 0 {
            return Ok("0-1_D3".to_owned());
        } else {
            if level_id == 0x7a {
                // FUN_020173c0(0xd,1);
                // Enemy Check, aka Museum
                return Ok("ene_check_".to_owned());
            } else if level_id == 0x7b {
                return Ok("koopa3".to_owned());
            } else if level_id == 0x7c {
                return Ok("koopa2".to_owned());
            } else if level_id == 0x7d {
                return Ok("kuppa".to_owned());
            } else if level_id == 0x7e {
                return Ok("lastback".to_owned());
            } else if level_id == 0x7f {
                return Ok("0x7f unknown multi".to_owned());
            }
        }
        if level_id > 100 {
            return Ok(">99 unknown multi".to_owned());
        }
        const LEVEL_ARRAY_ADDR: u32 = 0x0d8e58; //0x020d8e58
        let offset = level_id * 4; // u32 = 4 bytes
        let array_internal_address = LEVEL_ARRAY_ADDR + offset;
        if let Some(arm9_binary) = &self.loaded_arm9 {
            let mut rdr: Cursor<&Vec<u8>> = Cursor::new(arm9_binary);
            rdr.set_position(array_internal_address as u64);
            let string_address: u32 = match utils::read_address(&mut rdr) {
                Some(s) => s,
                None => {
                    let err_msg = "Failed to get string address in level name retrieval".to_owned();
                    log_write(err_msg.clone(), LogLevel::ERROR);
                    return Err(err_msg)
                },
            };
            rdr.set_position(string_address as u64);
            let level_name = utils::read_c_string(&mut rdr);
            Ok(level_name)
        } else {
            Err("ERROR, NO BINARY".to_owned())
        }
    }
    
    pub fn load_level(&mut self, world_index: u32, level_index: u32, map_index: u32) -> Result<(),String> {
        log_write(format!("Loading World {} Level {} Map {}",&world_index+1,&level_index+1,&map_index+1), LogLevel::LOG);
        let map_index_store = self.map_index; // Backup
        self.map_index = Some(map_index as usize);
        let mut initial_level_name = self.get_level_filename(&world_index, &level_index);
        initial_level_name.push_str(".crsb");
        let crsb_path = nitrofs_abs(&self.export_folder, &initial_level_name);
        let crsb = CourseInfo::new(&crsb_path,&format!("Course {}-{}",world_index+1,level_index+1));
        log_write(format!("Loaded Course '{}' from '{}'",&crsb.label,&crsb.src_filename), LogLevel::LOG);
        if (map_index as usize) >= crsb.level_map_data.len() {
            let err_msg = format!("map_index was out of bounds in load_level: '{}' >= '{}'",map_index,crsb.level_map_data.len());
            log_write(&err_msg, LogLevel::ERROR);
            // Revert
            self.map_index = map_index_store;
            return Err(err_msg);
        }
        let mut map_name = crsb.level_map_data[map_index as usize].map_filename_noext.clone();
        let noext_name = map_name.clone();
        let loaded_course_store = self.loaded_course.clone(); // Backup
        self.loaded_course = crsb;
        map_name.push_str(".mpdz");
        let map_path = nitrofs_abs(&self.export_folder, &map_name);
        let loaded_map_res = match MapData::new(&map_path,&self.export_folder) {
            Ok(x) => x,
            Err(e) => {
                let err_msg = format!("Failed to load MapData: '{}'",e);
                log_write(&err_msg, LogLevel::ERROR);
                // Revert
                self.map_index = map_index_store;
                self.loaded_course = loaded_course_store;
                return Err(err_msg);
            }
        };

        self.loaded_map = loaded_map_res;
        self.loaded_map.map_name = noext_name;

        let seg_count = &self.loaded_map.segments.len();
        let mapped: Vec<String> = self.loaded_map.segments.iter().map(|x| x.header()).collect();
        let mapped: String = mapped.join(", ");
        log_write(format!("Loaded Map '{}' with {} DataSegments: {}",&self.loaded_map.src_file,seg_count,mapped), LogLevel::LOG);
        
        // Do it manually the first time, don't wait for refresh
        self.update_graphics_from_mapdata();
        Ok(()) // Could something useful be returned?
    }

    pub fn get_render_archive(&mut self, archive_name_local: &String) -> &RenderArchive {
        if self.loaded_archives.contains_key(archive_name_local) {
            let arc_opt = self.loaded_archives.get(archive_name_local).expect("Error with RenderArchive get");
            arc_opt
        } else {
            let archive_name_full = nitrofs_abs(&self.export_folder, archive_name_local).display().to_string();
            let rarc = RenderArchive::new(archive_name_full, &self.export_folder);
            self.loaded_archives.insert(archive_name_local.clone(), rarc);
            let ret = self.loaded_archives.get(archive_name_local).expect("Error with RenderArchive get post creation");
            ret
        }
    }

    /// Copies data from MapData to graphics engine
    pub fn update_graphics_from_mapdata(&mut self) {
        // Initialize palettes //
        let mut pal_index: usize = 0;
        const UNIPAL_ADDR: u64 = 0x000d6f40;
        if let Some(arm9_binary) = &self.loaded_arm9 {
            let mut cur = Cursor::new(arm9_binary);
            cur.set_position(UNIPAL_ADDR);
            let pal = Palette::from_cur(&mut cur,16);
            self.bg_palettes[pal_index] = pal;
        } else {
            log_write("Could not load ARM9 to get universal palette", LogLevel::ERROR);
        }
        pal_index += 1;

        // BG loop //
        for which in 1..4_u8 { // This is 1,2,3; 4 is excluded
            let bg: Option<&mut BackgroundData> = self.loaded_map.get_background(which);
            if let Some(bg_data) = bg {
                // Palette
                if let Some(palette) = bg_data.get_pltb_mut().cloned() {
                    bg_data._pal_offset = pal_index as u8 - 1; // -1 to deal with universal palette
                    for p in &palette.palettes {
                        if pal_index < 16 {
                            self.bg_palettes[pal_index] = *p;
                        }
                        // else { // For some reason, there's more. But not used?
                        //     log_write(format!("Palette Overflow, discarding"), LogLevel::WARN);
                        // }
                        pal_index += 1;
                    }
                }
                // Setting to specific graphic memory
                // It is one way, copy it
                if which == 1 {
                    self.bg_layer_1 = Some(bg_data.clone());
                } else if which == 2 {
                    self.bg_layer_2 = Some(bg_data.clone());
                } else if which == 3 {
                    self.bg_layer_3 = Some(bg_data.clone());
                } else {
                    log_write(format!("Unusual which_bg in update_graphics_from_map: {}",which), LogLevel::ERROR);
                }
            } else {
                //log_write(format!("Did not get BG from get_background in graphics update"), LogLevel::WARN);
            }
        }
        // SETD (Sprites) //
        self.level_sprites.clear();
        if let Some(setd) = self.loaded_map.get_setd() {
            for sprite in &setd.sprites {
                // Copy data, it is one way
                self.level_sprites.push(sprite.clone());
            }
        }

        // GRAD (Background gradient) //
        if let Some(grad) = self.loaded_map.get_grad() {
            self.gradient_data = Some(grad.clone());
        }

        // PATH (Paths) //
        if let Some(path) = self.loaded_map.get_path() {
            self.path_data = Some(path.clone());
        }
    }

    pub fn update_sprite_metadata(&mut self, meta: &HashMap<u16,SpriteMetadata>) {
        self.sprite_metadata_copy = meta.clone();
    }

    pub fn get_loaded_sprite_by_uuid(&self, uuid: &Uuid) -> Option<&LevelSprite> {
        for sprite in &self.level_sprites {
            if sprite.uuid == *uuid {
                return Some(sprite);
            }
        }
        Option::None
    }

    pub fn get_selected_exit_mut(&mut self) -> Option<&mut MapExit> {
        let Some(selected_exit_uuid) = self.course_settings.selected_exit else {
            return Option::None;
        };
        let Some(selected_map_index) = self.course_settings.selected_map else {
            return Option::None;
        };
        if selected_map_index >= self.loaded_course.level_map_data.len() {
            self.course_settings.selected_map = Option::None;
            log_write("Selected map index out of bounds", LogLevel::WARN);
        }
        let selected_map = &mut self.loaded_course.level_map_data[selected_map_index];
        let map_exit = selected_map.get_exit(&selected_exit_uuid);
        map_exit
    }

}
