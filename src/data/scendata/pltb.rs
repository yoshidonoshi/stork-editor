// PLTB data segment within SCEN segments

use crate::{data::{types::Palette, Compilable}, engine::compression::segment_wrap};

use super::{info::ScenInfoData, ScenSegment};

#[derive(Clone,Debug,PartialEq)]
pub struct PltbData {
    pub palettes: Vec<Palette>
}

impl PltbData {
    pub fn from_pal_vec(input: Vec<Palette>) -> Self {
        Self {
            palettes: input
        }
    }
}

impl ScenSegment for PltbData {
    fn compile(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        self.palettes.iter().flat_map(|palette| palette.compile()).collect()
    }

    fn wrap(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        segment_wrap(self.compile(Option::None), self.header())
    }

    fn header(&self) -> String {
        String::from("PLTB")
    }
}
