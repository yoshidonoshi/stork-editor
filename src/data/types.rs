use std::{fmt::{self, Debug}, io::Cursor};

use egui::{Color32, TextureHandle};
use strum::EnumIter;

use crate::utils::{self, log_write, LogLevel};

use super::{segments::DataSegment, Compilable};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PalColor {
    pub color: Color32,
    pub _short: u16,
    pub _addr: u32
}
impl Default for PalColor {
    fn default() -> Self {
        Self { color: Color32::RED, _short: 0xBEEF, _addr: 0xDEADBEEF }
    }
}
impl fmt::Display for PalColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PalColor {{ color: {:?}, short: 0x{:0>4X}, addr: 0x{:0>8X} }}",self.color,self._short,self._addr)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub colors: [PalColor; 256],
    pub _pal_len: usize
}
impl fmt::Display for Palette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s: String = String::from("Palette { ");
        for p in self.colors {
            s.push_str(&p.to_string());
            s.push_str(", ");
        }
        s.push('}');
        write!(f, "{}", s)
    }
}
impl Default for Palette {
    fn default() -> Self {
        Self { colors: [PalColor::default();256], _pal_len: 0 }
    }
}
impl Palette {
    pub fn from_segment_index(source: &DataSegment, index: u32, pal_len: usize) -> Self {
        let data_header_str: String = utils::header_to_string(&source.header);
        if data_header_str != "PLTB" {
            log_write(format!("DataSegment was {} not PLTB, proceed with caution",&data_header_str), LogLevel::Error);
        }
        let mut cols: [PalColor; 256] = [PalColor::default(); 256];
        let internal_position = index * 16;
        let internal_data = &source.internal_data;
        let mut cur: Cursor<&Vec<u8>> = Cursor::new(internal_data);
        cur.set_position(internal_position as u64);
        let mut i: usize = 0;
        while i < pal_len {
            let short: u16 = cur.read_u16::<LittleEndian>().unwrap();
            let color = utils::color_from_u16(&short);
            cols[i].color = color;
            i += 1;
        }
        Self {
            colors: cols,
            _pal_len: pal_len
        }
    }
    pub fn from_cursor(rdr: &mut Cursor<&[u8]>, pal_len: usize) -> Self {
        let mut cols: [PalColor; 256] = [PalColor::default(); 256];
        let mut i: usize = 0;
        while i < pal_len {
            let short: u16 = rdr.read_u16::<LittleEndian>().unwrap();
            let color = utils::color_from_u16(&short);
            cols[i].color = color;
            cols[i]._short = short;
            cols[i]._addr = rdr.position() as u32;
            i += 1;
        }
        Self {
            colors: cols,
            _pal_len: pal_len
        }
    }
}

impl Compilable for Palette {
    fn compile(&self) -> Vec<u8> {
        // Confirmed this worked with the first 1-1 pull
        let mut comp: Vec<u8> = vec![];
        let mut i: usize = 0;
        while i < self._pal_len {
            let cur_col = &self.colors[i];
            let _ = comp.write_u16::<LittleEndian>(cur_col._short);
            i += 1;
        }
        comp
    }
}

/// This is the record stored within MPBZ data. 
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct MapTileRecordData {
    pub tile_id: u16,
    pub palette_id: u16,
    pub flip_h: bool,
    pub flip_v: bool
}
impl fmt::Display for MapTileRecordData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MapTileRecordData [ tile_id: 0x{:X}, palette_id: 0x{:X}, flip_h: {}, flip_v: {} ]",
            self.tile_id, self.palette_id, self.flip_h, self.flip_v)
    }
}
impl Compilable for MapTileRecordData {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let short_val: u16 = self.to_short();
        let _ = comp.write_u16::<LittleEndian>(short_val);
        comp
    }
}
impl MapTileRecordData {
    // http://problemkaputt.de/gbatek.htm#lcdvrambgscreendataformatbgmap
    pub fn new(short: u16) -> Self {
        let flip_v = ((short >> 11) % 2) == 1;
        let flip_h = ((short >> 10) % 2) == 1;
        let palette_id = short >> 12;
        let tile_id = short & 0b1111111111;
        Self {
            flip_h,
            flip_v,
            palette_id,
            tile_id
        }
    }
    pub fn to_short(self) -> u16 {
        let mut short_val: u16 = self.tile_id | ((self.flip_h as u16) << 10) | ((self.flip_v as u16) << 11);
        // Palette Id is unused in 256 mode
        // https://problemkaputt.de/gbatek.htm#lcdvrambgscreendataformatbgmap
        if self.palette_id <= 15 { // 16 color mode, so go for it
            short_val |= self.palette_id << 12;
        }
        short_val
    }
    pub fn get_render_pal_id(&self, layer_pal_offset: u8, color_mode: u32) -> usize {
        let mut pal_index = self.palette_id as usize;
        pal_index += layer_pal_offset as usize;
        // Pretty sure 0x2 is this
        if color_mode == 0x0 || color_mode == 0x2 {
            // Universal palette
            // "The following is an overflow-less "short += 0x1000; // 0201c730 ?""
            pal_index += 1;
        } else if color_mode == 0x1 {
            // Do nothing
        } else {
            log_write(format!("Unusual color mode in get_render_pal_id: {}",color_mode), LogLevel::Warn);
            // I think its color16
            pal_index += 1;
        }
        pal_index
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, EnumIter)]
pub enum CurrentLayer {
    BG1 = 1,
    BG2 = 2,
    BG3 = 3,
    Sprites = 4,
    Collision = 5,
    Paths = 6,
    Triggers = 7
}


pub type TileCache = Vec<Vec<Option<TextureHandle>>>;

pub fn wipe_tile_cache(tc: &mut TileCache) {
    for subarr in tc {
        for value in subarr {
            *value = None;
        }
    }
}

pub fn get_cached_texture(tc: &TileCache, global_palette_index: usize, tile_index: usize) -> Option<&TextureHandle> {
    if global_palette_index >= 16 {
        log_write(format!("texture cache: global_palette_index out of bounds: {}",global_palette_index), utils::LogLevel::Error);
        return Option::None;
    }
    if tile_index >= 1024 {
        log_write(format!("texture cache: tile_index out of bounds: {}",tile_index), utils::LogLevel::Error);
        return Option::None;
    }
    tc[global_palette_index][tile_index].as_ref()
}

pub fn set_cached_texture(tc: &mut TileCache, global_palette_index: usize, tile_index: usize, tex: TextureHandle) {
    tc[global_palette_index][tile_index] = Some(tex);
}
