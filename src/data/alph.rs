// This represents the ALPH data. This data is used to set BLDCNT and BLDALPHA,
//   usually for the purpose of making a background transparent

use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{engine::compression::segment_wrap, utils::{self, log_write, LogLevel}};

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
    pub fn new(byte_data: &[u8]) -> Option<Self> {
        let mut rdr = Cursor::new(byte_data);
        let cnt_res = match rdr.read_u16::<LittleEndian>() {
            Err(error) => {
                log_write(format!("Failed to get BLDCNT: '{error}'"), LogLevel::ERROR);
                return None;
            }
            Ok(cnt_res) => cnt_res,
        };
        Some(Self {
            bldcnt: cnt_res,
            bldalpha: utils::read_u16(&mut rdr)?
        })
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
        segment_wrap(comp_bytes, "ALPH".to_owned())
    }

    fn header(&self) -> String {
        String::from("ALPH")
    }
}
