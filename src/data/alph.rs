// This represents the ALPH data. This data is used to set BLDCNT and BLDALPHA,
//   usually for the purpose of making a background transparent

use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{engine::compression::segment_wrap, utils::{log_write, LogLevel}};

use super::TopLevelSegment;

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct AlphaData {
    pub bldcnt: u16,
    pub bldalpha: u16
}
impl Default for AlphaData {
    fn default() -> Self {
        Self { bldcnt: 0xffff, bldalpha: 0xffff }
    }
}

impl AlphaData {
    pub fn new(byte_data: &Vec<u8>) -> Self {
        let mut ret = AlphaData::default();
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(byte_data);
        let cnt_res = rdr.read_u16::<LittleEndian>();
        if cnt_res.is_err() {
            log_write(format!("Failed to get BLDCNT: '{}'",cnt_res.unwrap_err()), LogLevel::ERROR);
            return ret;
        }
        ret.bldcnt = cnt_res.unwrap();
        ret.bldalpha = rdr.read_u16::<LittleEndian>().expect("Should read ALPH second u16");
        ret
    }
}

impl TopLevelSegment for AlphaData {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u16::<LittleEndian>(self.bldcnt);
        let _ = comp.write_u16::<LittleEndian>(self.bldalpha);
        comp
    }

    fn wrap(&self) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile();
        segment_wrap(&comp_bytes, "ALPH".to_owned())
    }

    fn header(&self) -> String {
        String::from("ALPH")
    }
}
