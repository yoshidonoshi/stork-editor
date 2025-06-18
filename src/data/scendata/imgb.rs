use crate::engine::compression::segment_wrap;

use super::{info::ScenInfoData, ScenSegment};

#[derive(Debug,Clone,PartialEq)]
pub struct ImgbData {
    pub pixel_tiles: Vec<u8>
}

impl ImgbData {
    pub fn new(byte_data: Vec<u8>) -> Self {
        Self {
            pixel_tiles: byte_data,
        }
    }
}

impl ScenSegment for ImgbData {
    fn compile(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        self.pixel_tiles.clone()
    }

    fn wrap(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        segment_wrap(self.compile(info), self.header())
    }

    fn header(&self) -> String {
        String::from("IMGB")
    }
}
