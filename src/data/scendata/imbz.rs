use crate::engine::compression::{lamezip77_lz10_decomp, segment_wrap};

use super::ScenSegment;

#[derive(Debug,Clone,PartialEq)]
pub struct ImbzData {
    pub pixel_tiles: Vec<u8>
}

impl ImbzData {
    pub fn new(byte_data_compressed: &Vec<u8>) -> Self {
        let byte_data = lamezip77_lz10_decomp(byte_data_compressed);
        Self {
            pixel_tiles: byte_data
        }
    }
}

impl ScenSegment for ImbzData {
    fn compile(&self, _info: Option<&super::info::ScenInfoData>) -> Vec<u8> {
        self.pixel_tiles.clone()
    }

    fn wrap(&self, info: Option<&super::info::ScenInfoData>) -> Vec<u8> {
        let compressed = self.compile(info);
        segment_wrap(&compressed, self.header())
    }

    fn header(&self) -> String {
        String::from("IMBZ")
    }
}
