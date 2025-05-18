use crate::engine::compression::segment_wrap;

use super::TopLevelSegment;

#[derive(Debug,Clone,PartialEq)]
pub struct BrakData {
    pub raw_bytes: Vec<u8>
}
impl Default for BrakData {
    fn default() -> Self {
        Self { raw_bytes: Vec::new() }
    }
}
impl BrakData {
    pub fn new(byte_data: &Vec<u8>) -> Self {
        let mut brak = BrakData::default();
        brak.raw_bytes = byte_data.clone();
        brak
    }
    // pub fn test_vs_raw(&self,byte_data: &Vec<u8>) {
    //     compare_vector_u8s(&self.raw_bytes, byte_data);
    // }
}

impl TopLevelSegment for BrakData {
    fn compile(&self) -> Vec<u8> {
        self.raw_bytes.clone()
    }

    fn wrap(&self) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile();
        segment_wrap(&comp_bytes, self.header())
    }

    fn header(&self) -> String {
        String::from("BRAK")
    }
}
