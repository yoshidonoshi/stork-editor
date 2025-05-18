use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::engine::compression::{lamezip77_lz10_recomp, segment_wrap};

use super::{info::ScenInfoData, ScenSegment};

#[derive(Clone,Debug,PartialEq)]
pub struct AnmzDataSegment {
    pub frame_count: u8,
    pub unk1: u8,
    pub unk2: u16,
    pub vram_offset: u16,
    // There are two bytes inbetween here, likely padding as usually 0s
    pub frame_holds: Vec<u8>,
    pub pixeltiles: Vec<u8>,
    pub _raw_decomp: Vec<u8> // Until recompilation is added
}

impl Default for AnmzDataSegment {
    fn default() -> Self {
        Self {
            frame_count: 0,
            unk1: 0, unk2: 0,
            vram_offset: 0xffff,
            frame_holds: Vec::new(),
            pixeltiles: Vec::new(),
            _raw_decomp: Vec::new()
        }
    }
}

impl AnmzDataSegment {
    pub fn from_decomp(an_decomp: &Vec<u8>) -> AnmzDataSegment {
        let decomp_len: usize = an_decomp.len();
        //println!("Creating ANMZ from decomp with size of 0x'{:X}' bytes",decomp_len);
        let mut anmz = AnmzDataSegment::default();
        anmz._raw_decomp = an_decomp.clone();
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(an_decomp);
        anmz.frame_count = rdr.read_u8().unwrap();
        anmz.unk1 = rdr.read_u8().unwrap();
        anmz.unk2 = rdr.read_u16::<LittleEndian>().unwrap();
        anmz.vram_offset = rdr.read_u16::<LittleEndian>().unwrap();
        let _ = rdr.read_u8(); // Padding most likely
        let _ = rdr.read_u8();
        let mut frame_index: usize = 0;
        while frame_index < anmz.frame_count as usize {
            anmz.frame_holds.push(rdr.read_u8().unwrap());
            frame_index += 1;
        }
        // Pad to 4 bytes
        while rdr.position() % 4 != 0 {
            let _ = rdr.read_u8();
        }

        // Ends once is it EQUAL to length
        while (rdr.position() as usize) < decomp_len {
            let val = rdr.read_u8();
            if val.is_err() {
                println!("Error encountered when reading u8s for ANMZ");
                return anmz;
            }
            anmz.pixeltiles.push(val.unwrap());
        }
        anmz
    }
}

impl ScenSegment for AnmzDataSegment {
    fn compile(&self, _info: &Option<ScenInfoData>) -> Vec<u8> {
        self._raw_decomp.clone()
    }

    fn wrap(&self, info: &Option<ScenInfoData>) -> Vec<u8> {
        let compiled = self.compile(info);
        let compressed = lamezip77_lz10_recomp(&compiled);
        segment_wrap(&compressed, self.header())
    }

    fn header(&self) -> String {
        String::from("ANMZ")
    }
}
