use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{engine::compression::{lamezip77_lz10_decomp, lamezip77_lz10_recomp, segment_wrap}, utils::{log_write, LogLevel}};

use super::{types::MapTileRecordData, TopLevelSegment};


#[derive(Debug,Clone,PartialEq,Default)]
pub struct SoftRockBackdrop {
    pub x_offset: u16,
    pub y_offset: u16,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<MapTileRecordData>
}

impl SoftRockBackdrop {
    pub fn new(byte_data: &Vec<u8>) -> Self {
        let mut ret = SoftRockBackdrop::default();
        let byte_data = &lamezip77_lz10_decomp(byte_data);
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(byte_data);
        let first_res = rdr.read_u16::<LittleEndian>();
        if first_res.is_err() {
            log_write(format!("Failed to get first result in SoftRockBackdrop: '{}'",first_res.unwrap_err()), LogLevel::ERROR);
            return ret;
        }
        ret.x_offset = first_res.unwrap();
        ret.y_offset = rdr.read_u16::<LittleEndian>().expect("BLKZ yOffset");
        ret.width = rdr.read_u16::<LittleEndian>().expect("BLKZ width");
        ret.height = rdr.read_u16::<LittleEndian>().expect("BLKZ height");

        let end_len = byte_data.len() as u64;
        while rdr.position() < end_len {
            let tile_short = rdr.read_u16::<LittleEndian>().expect("BLKZ tile read");
            ret.tiles.push(MapTileRecordData::new(&tile_short));
        }
        let calced_len = (ret.width as usize) * (ret.height as usize);
        if calced_len != ret.tiles.len() {
            log_write(format!("Mismatch in height*width to tile len: {} vs {}",calced_len,ret.tiles.len()), LogLevel::ERROR);
        }

        ret
    }
}

impl TopLevelSegment for SoftRockBackdrop {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u16::<LittleEndian>(self.x_offset);
        let _ = comp.write_u16::<LittleEndian>(self.y_offset);
        let _ = comp.write_u16::<LittleEndian>(self.width);
        let _ = comp.write_u16::<LittleEndian>(self.height);
        for tile in &self.tiles {
            let short = tile.to_short();
            let _ = comp.write_u16::<LittleEndian>(short);
        }
        comp
    }

    fn wrap(&self) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile();
        let comp_bytes = lamezip77_lz10_recomp(&comp_bytes);
        segment_wrap(&comp_bytes, self.header())
    }

    fn header(&self) -> String {
        String::from("BLKZ")
    }
}
