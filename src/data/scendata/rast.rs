use crate::{engine::compression::segment_wrap, utils::{log_write, LogLevel}};

use super::{info::ScenInfoData, ScenSegment};

#[derive(Debug,Clone,PartialEq,Default)]
pub struct RastData {
    pub _raw: Vec<u8>
}
impl RastData {
    pub fn new(byte_data: &Vec<u8>) -> Self {
        log_write("RAST is unhandled, storing raw data to enable safe saving", LogLevel::WARN);
        let mut rast = RastData::default();
        rast._raw = byte_data.clone();
        rast
    }
}

impl ScenSegment for RastData {
    fn compile(&self, _info: &Option<ScenInfoData>) -> Vec<u8> {
        self._raw.clone()
    }

    fn wrap(&self, info: &Option<ScenInfoData>) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile(info);
        segment_wrap(&comp_bytes, self.header())
    }

    fn header(&self) -> String {
        String::from("RAST")
    }
}