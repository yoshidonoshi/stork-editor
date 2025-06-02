use crate::engine::compression::segment_wrap;

use super::{info::ScenInfoData, ScenSegment};

#[derive(Debug,Clone,PartialEq,Default)]
pub struct AnimatedPaletteData {
    pub _raw: Vec<u8>
}

impl AnimatedPaletteData {
    pub fn new(byte_data: &Vec<u8>) -> Self {
        Self {
            _raw: byte_data.clone()
        }
    }
}

impl ScenSegment for AnimatedPaletteData {
    fn compile(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        self._raw.clone()
    }

    fn wrap(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        let compiled = self.compile(info);
        segment_wrap(&compiled, self.header())
    }

    fn header(&self) -> String {
        String::from("PLAN")
    }
}