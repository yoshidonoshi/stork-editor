use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{engine::compression::segment_wrap, utils::{log_write, LogLevel}};

use super::{info::ScenInfoData, ScenSegment};


#[derive(Debug,Clone,Copy,PartialEq)]
pub struct ScrollData {
    pub left_velocity: i32,
    pub up_velocity: i32
}

impl Default for ScrollData {
    fn default() -> Self {
        Self {
            left_velocity: 0x00000000,
            up_velocity: 0x00000000
        }
    }
}

impl ScrollData {
    pub fn new(rdr: &mut Cursor<&Vec<u8>>) -> Self {
        let mut ret = ScrollData::default();
        let left_vel = rdr.read_i32::<LittleEndian>();
        if left_vel.is_err() {
            log_write(format!("Could not read Left Velocity: '{}'",left_vel.unwrap_err()), LogLevel::ERROR);
            return ret;
        }
        ret.left_velocity = left_vel.unwrap();
        ret.up_velocity = rdr.read_i32::<LittleEndian>().expect("Up Velocity SCRL");
        ret
    }
}

impl ScenSegment for ScrollData {
    fn compile(&self, _info: &Option<ScenInfoData>) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_i32::<LittleEndian>(self.left_velocity);
        let _ = comp.write_i32::<LittleEndian>(self.up_velocity);
        comp
    }

    fn wrap(&self, info: &Option<ScenInfoData>) -> Vec<u8> {
        segment_wrap(&self.compile(info), self.header())
    }

    fn header(&self) -> String {
        String::from("SCRL")
    }
}
