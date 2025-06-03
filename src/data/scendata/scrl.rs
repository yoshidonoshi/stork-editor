use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{engine::compression::segment_wrap, utils::{log_write, LogLevel}};

use super::{info::ScenInfoData, ScenSegment};


#[derive(Debug,Clone,Copy,PartialEq,Default)]
pub struct ScrollData {
    pub left_velocity: i32,
    pub up_velocity: i32
}

impl ScrollData {
    pub fn new(rdr: &mut Cursor<&Vec<u8>>) -> Self {
        let left_vel = match rdr.read_i32::<LittleEndian>() {
            Err(error) => {
                log_write(format!("Could not read Left Velocity: '{error}'"), LogLevel::ERROR);
                return ScrollData::default();
            }
            Ok(v) => v,
        };
        Self {
            left_velocity: left_vel,
            up_velocity: rdr.read_i32::<LittleEndian>().expect("Up Velocity SCRL")
        }
    }
}

impl ScenSegment for ScrollData {
    fn compile(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_i32::<LittleEndian>(self.left_velocity);
        let _ = comp.write_i32::<LittleEndian>(self.up_velocity);
        comp
    }

    fn wrap(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        segment_wrap(&self.compile(info), self.header())
    }

    fn header(&self) -> String {
        String::from("SCRL")
    }
}
