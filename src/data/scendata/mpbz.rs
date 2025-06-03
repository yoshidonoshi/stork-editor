// Map tiles. Has some possible extra data at the top

use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{data::types::MapTileRecordData, engine::compression::{lamezip77_lz10_recomp, segment_wrap}, utils::{compare_vector_u8s, log_write, LogLevel}};

use super::{info::ScenInfoData, ScenSegment};

#[derive(Clone,Debug,PartialEq)]
pub struct MapTileDataSegment {
    pub tiles: Vec<MapTileRecordData>,
    pub tile_offset: u16,
    pub bottom_trim: u16,
}

impl MapTileDataSegment {
    pub fn from_decomped_vec(mp_decomp: &Vec<u8>, layer_width: u16) -> Self {
        let mut mpbz_vec: Vec<MapTileRecordData> = Vec::new();
        let mut count_tiles: u32 = mp_decomp.len() as u32 / 2;
        let tile_offset: u16;
        let bottom_trim: u16;
        let mut rdr2: Cursor<&Vec<u8>> = Cursor::new(mp_decomp);
        // Check for offsets
        let first = rdr2.read_u16::<LittleEndian>().unwrap();
        if first == 0xffff {
            // There's special data
            tile_offset = rdr2.read_u16::<LittleEndian>().unwrap();
            bottom_trim = rdr2.read_u16::<LittleEndian>().unwrap();
            let offset: u32 = (layer_width * tile_offset) as u32;
            let blank = MapTileRecordData::new(&0x0000);
            for _ in 0..offset {
                mpbz_vec.push(blank);
            }
            count_tiles -= 3; // Undo the 3 tiles worth of data read
        } else {
            // It was normal, reset it back to the beginning
            tile_offset = 0;
            bottom_trim = 0;
            rdr2.set_position(0); 
        }
        // Now load the tiles themselves
        let mut tile_index = 0;
        while tile_index < count_tiles {
            let short: u16 = rdr2.read_u16::<LittleEndian>().unwrap();
            let tile = MapTileRecordData::new(&short);
            // UPDATED: STOP MODIFYING THE TILES THEMSELVES //
            // The following is an overflow-less "short += 0x1000; // 0201c730 ?"
            // Applies in 16 palette color modes, probably because of universal palette
            // if color_mode == 0x0 {
            //     tile.palette_id += 1; // Because of universal palette
            // }
            mpbz_vec.push(tile); // Hand it over
            tile_index += 1;
        }
        Self {
            tiles: mpbz_vec,
            bottom_trim,
            tile_offset
        }
    }

    #[allow(dead_code)]
    pub fn test_against_raw_decomp(&self, info: Option<&ScenInfoData>, raw_decomp: &Vec<u8>) {
        log_write("Doing MPBZ recompilation test",LogLevel::DEBUG);
        let comp = self.compile(info);
        compare_vector_u8s(&comp, raw_decomp);
    }

    pub fn increase_width(&mut self, old_width: u16, increase_by: usize) {
        let mut idx: usize = old_width as usize;
        // Do loop here
        while idx <= self.tiles.len() {
            for _ in 0..increase_by {
                self.tiles.insert(idx, MapTileRecordData::new(&0x0000));
            }
            idx = idx + (old_width as usize) + increase_by;
        }
    }

    pub fn decrease_width(&mut self, old_width: u16, decrease_by: usize) {
        let mut idx: i32 = old_width as i32 -1;

        while idx < self.tiles.len() as i32 {
            for _ in 0..decrease_by {
                self.tiles.remove(idx as usize);
                idx -= 1;
            }
            idx += old_width as i32;
        }
    }

    pub fn change_height(&mut self, new_height: u16, width: u16) {
        let new_len = (new_height as u32) * (width as u32);
        self.tiles.resize(new_len as usize, MapTileRecordData::new(&0x0000));
    }
}

impl ScenSegment for MapTileDataSegment {
    fn compile(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        let Some(info) = info else {
            // Probably do Err for this in the future, but this is basically fatal
            log_write("Missing info parameter in MapTileDataSegment compiler", LogLevel::FATAL);
            return Vec::new();
        };
        let mut comp: Vec<u8> = vec![];
        let mut index: usize = 0;
        if self.bottom_trim > 0 || self.tile_offset > 0 {
            comp.push(0xff);
            comp.push(0xff);
            let _ = comp.write_u16::<LittleEndian>(self.tile_offset);
            let _ = comp.write_u16::<LittleEndian>(self.bottom_trim);
            index = (self.tile_offset as usize) * (info.layer_width as usize);
        }
        let tiles_len: usize = self.tiles.len();
        while index < tiles_len {
            // Needs to be cloned
            let tile_compiled = self.tiles[index].to_short();
            let _ = comp.write_u16::<LittleEndian>(tile_compiled);
            index += 1;
        }
        comp
    }

    fn wrap(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        if info.is_none() {
            // Again, maybe change all these to Err, but this is catastrophic
            log_write("Missing info parameter in MapTileDataSegment wrapper", LogLevel::FATAL);
            return Vec::new();
        }
        let comped = self.compile(info);
        let mpbz_compressed = lamezip77_lz10_recomp(&comped);
        segment_wrap(&mpbz_compressed, self.header())
    }

    fn header(&self) -> String {
        String::from("MPBZ")
    }
}
