// This is a container for MPDZ files
// 
// It represents an entire map, primarily including
// the backgrounds, but also objects and more
// 
// It is not read from constantly by the graphics engine,
// rather it is copied on demand for performance

use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use byteorder::{LittleEndian, ReadBytesExt};
use uuid::Uuid;
use crate::engine::compression::{lamezip77_lz10_recomp, segment_wrap_u32};
use crate::load::SPRITE_METADATA;
use crate::utils::{header_to_string, log_write};
use crate::{engine::compression, utils::{self, LogLevel}};

use super::alph::AlphaData;
use super::area::TriggerData;
use super::backgrounddata::BackgroundData;
use super::blkz::SoftRockBackdrop;
use super::brak::BrakData;
use super::grad::GradientData;
use super::path::PathDatabase;
use super::segments::DataSegment;
use super::sprites::{LevelSprite, LevelSpriteSet};
use super::types::MapTileRecordData;
use super::{GenericTopLevelSegment, TopLevelSegment};

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone,PartialEq,Debug)]
pub enum TopLevelSegmentWrapper {
    SETD(LevelSpriteSet),
    SCEN(BackgroundData),
    GRAD(GradientData),
    AREA(TriggerData),
    PATH(PathDatabase),
    ALPH(AlphaData),
    BLKZ(SoftRockBackdrop),
    BRAK(BrakData),
    Unknown(GenericTopLevelSegment)
}

impl TopLevelSegment for TopLevelSegmentWrapper {
    fn compile(&self) -> Vec<u8> {
        match self {
            Self::SCEN(scen) => scen.compile(),
            Self::SETD(setd) => setd.compile(),
            Self::GRAD(grad) => grad.compile(),
            Self::AREA(area) => area.compile(),
            Self::PATH(path) => path.compile(),
            Self::ALPH(alph) => alph.compile(),
            Self::BLKZ(blkz) => blkz.compile(),
            Self::BRAK(brak) => brak.compile(),
            Self::Unknown(unkn) => unkn.compile()
        }
    }

    fn wrap(&self) -> Vec<u8> {
        match self {
            Self::SCEN(scen) => scen.wrap(),
            Self::SETD(setd) => setd.wrap(),
            Self::GRAD(grad) => grad.wrap(),
            Self::AREA(area) => area.wrap(),
            Self::PATH(path) => path.wrap(),
            Self::ALPH(alph) => alph.wrap(),
            Self::BLKZ(blkz) => blkz.wrap(),
            Self::BRAK(brak) => brak.wrap(),
            Self::Unknown(unkn) => unkn.wrap()
        }
    }

    fn header(&self) -> String {
        match self {
            Self::SCEN(scen) => scen.header(),
            Self::SETD(setd) => setd.header(),
            Self::GRAD(grad) => grad.header(),
            Self::AREA(area) => area.header(),
            Self::PATH(path) => path.header(),
            Self::ALPH(alph) => alph.header(),
            Self::BLKZ(blkz) => blkz.header(),
            Self::BRAK(brak) => brak.header(),
            Self::Unknown(unkn) => unkn.header()
        }
    }
}

/// This exists purely as an interface to the file itself
#[derive(Clone,PartialEq)]
pub struct MapData {
    pub src_file: String,
    pub map_name: String,
    pub segments: Vec<TopLevelSegmentWrapper>,
    pub uuid: Uuid,
    pub unhandled_headers: Vec<String>
}
impl Default for MapData {
    fn default() -> Self {
        Self {
            src_file: String::from("ERROR"),
            map_name: String::from("ERROR"),
            segments: Vec::new(),
            uuid: Uuid::new_v4(),
            unhandled_headers: Vec::new()
        }
    }
}
impl MapData {
    pub fn new(filename_abs: &PathBuf, project_folder: &Path) -> Result<Self, MapDataError> {
        let mut ret: MapData = MapData {
            src_file: filename_abs.to_string_lossy().to_string(),
            ..Default::default()
        };
        if matches!(fs::exists(filename_abs), Err(_) | Ok(false)) {
            let file_exists_err = MapDataError::FileNotExist(filename_abs.display().to_string());
            log_write(&file_exists_err, LogLevel::Error);
            return Err(file_exists_err);
        }
        let file_bytes: Vec<u8> = compression::decompress_file(filename_abs);
        let mut rdr = Cursor::new(&file_bytes);
        let file_header = match rdr.read_u32::<LittleEndian>() {
            Err(_) => {
                let master_header_err = MapDataError::MasterHeaderNotFound;
                log_write(&master_header_err, LogLevel::Error);
                return Err(master_header_err);
            }
            Ok(h) => h,
        };
        // It's 3 characters long, not 4
        let header = header_to_string(&file_header);
        let mut header_iter = header.chars().take(3);
        let header_chars: [char; 3] = std::array::from_fn(|_| header_iter.next().unwrap());
        if "SET".chars().ne(header_chars) {
            let set_missing_msg = MapDataError::HeaderWasntSet(header_chars);
            log_write(&set_missing_msg, LogLevel::Error);
            return Err(set_missing_msg);
        }
        let _ = rdr.read_u32::<LittleEndian>().unwrap();
        let mut segments: Vec<DataSegment> = vec![];
        let file_end_pos: u64 = file_bytes.len() as u64;
        while rdr.position() < file_end_pos {
            let section_head: u32 = rdr.read_u32::<LittleEndian>().unwrap();
            let section_size: usize = rdr.read_u32::<LittleEndian>().unwrap() as usize;
            let internal_vec = (0..section_size).map(|_| rdr.read_u8().unwrap()).collect();
            let cur_segment: DataSegment = DataSegment { header: section_head, internal_data: internal_vec };
            segments.push(cur_segment);
        }
        // We now have all the basic data segments
        for segment in &segments {
            let seg_header: u32 = segment.header;
            let seg_header = utils::header_to_string(&seg_header);
            log_write(format!("Parsing top level Segment '{}' with size 0x{:X}",seg_header,segment.internal_data.len()), LogLevel::Debug);
            match seg_header.as_str() {
                "SCEN" => {
                    if let Ok(bg) = BackgroundData::new(&segment.internal_data, project_folder) {
                        ret.segments.push(TopLevelSegmentWrapper::SCEN(bg));
                    } else {
                        let bg_fail_msg = MapDataError::FailedGenerateBackground;
                        log_write(&bg_fail_msg, LogLevel::Error);
                        return Err(bg_fail_msg);
                    }
                }
                "SETD" => {
                    let setd = LevelSpriteSet::new(&segment.internal_data);
                    let scount = setd.sprites.len();
                    log_write(format!("Loaded {}/0x{:X} Sprites for the level",scount,scount), LogLevel::Debug);
                    ret.segments.push(TopLevelSegmentWrapper::SETD(setd));
                }
                "GRAD" => {
                    let grad = match GradientData::new(&segment.internal_data) {
                        Some(g) => g,
                        None => {
                            log_write("Failed to load GRAD", LogLevel::Error);
                            continue;
                        },
                    };
                    ret.segments.push(TopLevelSegmentWrapper::GRAD(grad));
                }
                "AREA" => {
                    let area = TriggerData::new(&segment.internal_data);
                    ret.segments.push(TopLevelSegmentWrapper::AREA(area));
                }
                "PATH" => {
                    let path = PathDatabase::new(&segment.internal_data);
                    ret.segments.push(TopLevelSegmentWrapper::PATH(path));
                }
                "ALPH" => {
                    let alph = match AlphaData::new(&segment.internal_data) {
                        Some(a) => a,
                        None => {
                            log_write("Failed to load ALPH", LogLevel::Error);
                            continue;
                        },
                    };
                    ret.segments.push(TopLevelSegmentWrapper::ALPH(alph));
                }
                "BLKZ" => {
                    let blkz = match SoftRockBackdrop::new(&segment.internal_data) {
                        Some(b) => b,
                        None => {
                            log_write("failed to load BLKZ", LogLevel::Error);
                            continue;
                        },
                    };
                    ret.segments.push(TopLevelSegmentWrapper::BLKZ(blkz));
                }
                "BRAK" => {
                    let brak = BrakData::new(segment.internal_data.clone());
                    ret.segments.push(TopLevelSegmentWrapper::BRAK(brak));
                }
                _ => {
                    log_write(format!("Level DataSegment header '{}' unhandled",&seg_header), LogLevel::Warn);
                    let unkn = GenericTopLevelSegment::new(segment.internal_data.clone(), seg_header.clone());
                    ret.unhandled_headers.push(seg_header);
                    ret.segments.push(TopLevelSegmentWrapper::Unknown(unkn));
                }
            }
        } // End loop for segments

        Ok(ret)
    }

    pub fn get_background(&mut self, which_background: u8) -> Option<&mut BackgroundData> {
        for seg in &mut self.segments {
            if let TopLevelSegmentWrapper::SCEN(scen) = seg {
                if scen.get_info().expect("get_background info").which_bg == which_background {
                    return Some(scen);
                }
            }
        }
        Option::None
    }

    pub fn get_setd(&mut self) -> Option<&mut LevelSpriteSet> {
        for seg in &mut self.segments {
            if let TopLevelSegmentWrapper::SETD(setd) = seg {
                return Some(setd);
            }
        }
        Option::None
    }

    pub fn get_grad(&mut self) -> Option<&mut GradientData> {
        for seg in &mut self.segments {
            if let TopLevelSegmentWrapper::GRAD(grad) = seg {
                return Some(grad);
            }
        }
        Option::None
    }

    pub fn get_path(&mut self) -> Option<&mut PathDatabase> {
        for seg in &mut self.segments {
            if let TopLevelSegmentWrapper::PATH(p) = seg {
                return Some(p);
            }
        }
        Option::None
    }

    pub fn get_blkz(&self) -> Option<&SoftRockBackdrop> {
        for seg in &self.segments {
            if let TopLevelSegmentWrapper::BLKZ(b) = seg {
                return Some(b);
            }
        }
        Option::None
    }

    pub fn get_area(&self) -> Option<&TriggerData> {
        for seg in &self.segments {
            if let TopLevelSegmentWrapper::AREA(a) = seg {
                return Some(a);
            }
        }
        Option::None
    }

    pub fn get_area_mut(&mut self) -> Option<&mut TriggerData> {
        for seg in &mut self.segments {
            if let TopLevelSegmentWrapper::AREA(a) = seg {
                return Some(a);
            }
        }
        Option::None
    }

    pub fn get_bg_with_colz(&self) -> Option<u8> {
        for seg in &self.segments {
            if let TopLevelSegmentWrapper::SCEN(scen) = seg {
                // This SCEN has Collision!
                if scen.get_colz().is_some() {
                    // Return which BG it was that had COLZ
                    let which_bg = scen.get_info().expect("INFO guaranteed if there's COLZ").which_bg;
                    return Some(which_bg);
                }
            }
        }
        Option::None
    }

    /// Create the uncompressed interior data without header
    /// 
    /// Loops over the loaded segments and wraps each one (wrap containing compile),
    /// appending each compiled segment to an output byte array
    pub fn compile(&self) -> Vec<u8> {
        let mut compiled: Vec<u8> = Vec::new();
        for segment in &self.segments {
            let mut seg_comp = segment.wrap();
            compiled.append(&mut seg_comp);
        }
        compiled
    }

    /// Wrap with header, then compress the entire thing
    /// 
    /// Both compiles, wraps, and globally compresses the data, preparing it to
    /// be written to an MPDZ file
    pub fn package(&self) -> Vec<u8> {
        let interior = self.compile();
        let wrapped = segment_wrap_u32(interior, 0x00544553);
        lamezip77_lz10_recomp(&wrapped)
    }

    ////////////////////////////////////////////
    // Functions for updating the data itself //
    ////////////////////////////////////////////

    /// Move a sprite in the map data
    pub fn move_sprite(&mut self, sprite_uuid: Uuid, new_x: u16, new_y: u16) {
        let sprite_set = self.get_setd().expect("Expected SETD to exist");
        for spr in &mut sprite_set.sprites {
            if spr.uuid == sprite_uuid {
                spr.x_position = new_x;
                spr.y_position = new_y;
            }
        }
    }

    pub fn update_sprite_settings(&mut self, sprite_uuid: Uuid, new_settings: Vec<u8>) {
        let sprite_set = self.get_setd().expect("Expected SETD to exist");
        for spr in &mut sprite_set.sprites {
            if spr.uuid == sprite_uuid {
                if spr.settings_length as usize != new_settings.len() {
                    log_write(format!("Attempted to update sprite settings with vec len {}, standard len is {}",
                        new_settings.len(),spr.settings_length), LogLevel::Error);
                    return;
                }
                spr.settings = new_settings;
                return; // Consumed, break loop
            }
        }
    }

    pub fn add_sprite(&mut self, sprite: LevelSprite) -> Uuid {
        let uuid = sprite.uuid;
        self.get_setd().expect("Expected SETD to exist").sprites.push(sprite);
        uuid
    }

    pub fn add_new_sprite_at(&mut self, sprite_id: u16, x: u16, y:u16) -> Uuid {
        let Some(sprite_set) = self.get_setd() else {
            // This really shouldn't be possible
            log_write("SETD not loaded when placing sprite".to_owned(),LogLevel::Error);
            return Uuid::nil();
        };
        let Some(sprite_meta) = SPRITE_METADATA.get(&sprite_id) else {
            log_write(format!("No Sprite metadata found for 0x{sprite_id:X}"),LogLevel::Error);
            return Uuid::nil();
        };
        let new_sprite = LevelSprite::new(sprite_id, x, y, vec![0;sprite_meta.default_settings_len as usize]);
        let ret = new_sprite.uuid;
        sprite_set.sprites.push(new_sprite);
        ret
    }

    /// Return a cloned copy of a Sprite from the current level map
    pub fn get_sprite_by_uuid(&mut self, sprite_uuid: Uuid) -> Option<LevelSprite> {
            self.get_setd().expect("Expected SETD to exist")
                .sprites.iter().find(|&spr| spr.uuid == sprite_uuid).cloned()
    }

    /// Returns true if deleted successfully
    pub fn delete_sprite_by_uuid(&mut self, sprite_uuid: Uuid) -> bool {
        let sprite_set = self.get_setd().expect("Expected SETD to exist");
        sprite_set.sprites.iter()
            .position(|spr| spr.uuid == sprite_uuid)
            .map(|i| sprite_set.sprites.remove(i)).is_some()
    }

    /// Returns if it got successfully set or not
    pub fn set_col_tile(&mut self, which_background: u8, tile_index: u16, new_type: u8) -> bool {
        #[allow(clippy::manual_range_contains)]
        if which_background < 1 || which_background > 3 {
            log_write(format!("Extremely unusual which_background value in delete_bg_tile_by_map_index: {}",which_background), LogLevel::Error);
            return false
        }
        let Some(bg) = self.get_background(which_background) else {
            log_write(format!("Failed to get_background '{}' in delete_bg_tile_by_map_index",which_background), LogLevel::Error);
            return false
        };
        if let Some(col) = bg.get_colz_mut() {
            col.col_tiles[tile_index as usize] = new_type;
            true
        } else {
            false
        }
    }

    pub fn delete_bg_tile_by_map_index(&mut self, which_background: u8, map_index: u32) -> bool {
        #[allow(clippy::manual_range_contains)]
        if which_background < 1 || which_background > 3 {
            log_write(format!("Extremely unusual which_background value in delete_bg_tile_by_map_index: {}",which_background), LogLevel::Error);
            return false;
        }
        let Some(bg) = self.get_background(which_background) else {
            log_write(format!("Failed to get_background '{}' in delete_bg_tile_by_map_index",which_background), LogLevel::Error);
            return false;
        };
        if let Some(tiles_segment) = bg.get_mpbz_mut() {
            if (map_index as usize) > tiles_segment.tiles.len() {
                log_write(format!("Overflow in delete_bg_tile_by_map_index: {} >= {}",&map_index,&tiles_segment.tiles.len()), LogLevel::Error);
                return false;
            }
            let empty_record: MapTileRecordData = MapTileRecordData::default();
            tiles_segment.tiles[map_index as usize] = empty_record;
            // Ultimately the palette doesn't really matter since the tile is 0, transparent...
        }
        true
    }

    pub fn place_bg_tile_at_map_index(&mut self, which_background: u8, map_index: u32, tile: u16) -> bool {
        #[allow(clippy::manual_range_contains)]
        if which_background < 1 || which_background > 3 {
            log_write(format!("Extremely unusual which_background value in place_bg_tile_at_map_index: '{}'",which_background), LogLevel::Error);
            return false;
        }
        let Some(bg) = self.get_background(which_background) else {
            log_write(format!("Failed to get_background '{}' in place_bg_tile_at_map_index",which_background), LogLevel::Error);
            return false;
        };
        if let Some(tiles_segment) = bg.get_mpbz_mut() {
            if (map_index as usize) > tiles_segment.tiles.len() {
                // May be pasted out of bounds
                log_write(format!("Overflow in place_bg_tile_at_map_index {} >= {}",&map_index,&tiles_segment.tiles.len()), LogLevel::Error);
                return false;
            }
            tiles_segment.tiles[map_index as usize] = MapTileRecordData::new(tile);
        } else {
            log_write(format!("Could not find map tiles for bg '{}' in place_bg_tile_at_map_index",which_background), LogLevel::Error);
            return false;
        }
        true
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapDataError {
    FileNotExist(String),
    MasterHeaderNotFound,
    HeaderWasntSet([char; 3]),
    FailedGenerateBackground,
}
impl Display for MapDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MasterHeaderNotFound => f.write_str("Error getting master header from MapData"),
            Self::FileNotExist(path) => f.write_fmt(format_args!("File does not exist: {path}")),
            Self::HeaderWasntSet([a,b,c]) => f.write_fmt(format_args!("MapData master header was not 'SET', was instead '{a}{b}{c}'")),
            Self::FailedGenerateBackground => f.write_str("Failed to generate BackgroundData in MapData"),
        }
    }
}
impl Error for MapDataError {}
