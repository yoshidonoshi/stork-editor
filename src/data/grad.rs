use std::io::{Cursor, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{engine::compression::segment_wrap, utils::{self, log_write, read_fixed_string_cursor, LogLevel}};

use super::TopLevelSegment;

#[derive(Debug,Clone,PartialEq,Default)]
pub struct GradientData {
    // GINF
    pub color_count: u16,
    pub unknown1: i16, // Signed!
    pub unknown2: u16,
    _padding: u16, // Just in case it turns out to be something else
    pub y_offset: u32,
    // GCOL
    pub color_shorts: Vec<u16>
}
impl TopLevelSegment for GradientData {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        // GINF header
        let ginf_bytes = "GINF".as_bytes();
        let _ = comp.write(ginf_bytes);
        let ginf_size: u32 = 0xC;
        let _ = comp.write_u32::<LittleEndian>(ginf_size);
        // Actual internal data
        let _ = comp.write_u16::<LittleEndian>(self.color_count);
        let _ = comp.write_i16::<LittleEndian>(self.unknown1);
        let _ = comp.write_u16::<LittleEndian>(self.unknown2);
        let _ = comp.write_u16::<LittleEndian>(self._padding); // Just in case
        let _ = comp.write_u32::<LittleEndian>(self.y_offset);
        // GCOL header
        let gcol_bytes = "GCOL".as_bytes();
        let _ = comp.write(gcol_bytes);
        let colors_size: usize = self.color_shorts.len() * 2; // Each is 2 bytes
        if colors_size != (self.color_count * 2) as usize {
            log_write(format!("Mismatch in colors_size vs self.color_count in GRAD compile: {:X} vs {:X}",&colors_size,&self.color_count), LogLevel::Error);
        }
        let _ = comp.write_u32::<LittleEndian>(colors_size as u32);
        for color in &self.color_shorts {
            let _ = comp.write_u16::<LittleEndian>(*color);
        }
        comp
    }
    
    fn wrap(&self) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile();
        // It is not compressed
        segment_wrap(comp_bytes, "GRAD".to_owned())
    }

    fn header(&self) -> String {
        String::from("GRAD")
    }
}
impl GradientData {
    pub fn new(bytedata: &[u8]) -> Option<Self> {
        let mut ret = GradientData::default();
        let mut rdr = Cursor::new(bytedata);

        let ginf_header: String = read_fixed_string_cursor(&mut rdr, 4);
        if ginf_header != "GINF" {
            log_write(format!("Did not find GINF header, instead got '{}'",ginf_header), LogLevel::Error);
            return None;
        }
        let ginf_size = rdr.read_u32::<LittleEndian>().unwrap();
        if ginf_size != 0xc {
            log_write(format!("GINF was not 0xC bytes, was instead {:X}",ginf_size), LogLevel::Error);
            return None;
        }
        ret.color_count = utils::read_u16(&mut rdr)?; //rdr.read_u16::<LittleEndian>().unwrap();
        ret.unknown1 = utils::read_i16(&mut rdr)?;//rdr.read_i16::<LittleEndian>().unwrap();
        ret.unknown2 = utils::read_u16(&mut rdr)?;
        ret._padding = utils::read_u16(&mut rdr)?; // Just in case it's something else
        if ret._padding != 0x0000 {
            log_write(format!("GINF padding was not padding after all! Value was '{:X}', tell creator this",ret._padding), LogLevel::Warn);
        }
        ret.y_offset = rdr.read_u32::<LittleEndian>().unwrap();
        let gcol_header: String = read_fixed_string_cursor(&mut rdr, 4);
        if gcol_header != "GCOL" {
            log_write(format!("Did not find GCOL header, instead got '{}'",gcol_header), LogLevel::Error);
            return None;
        }
        let gcol_size: u32 = rdr.read_u32::<LittleEndian>().unwrap();
        if gcol_size / 2 != ret.color_count as u32 {
            log_write(format!("Mismatch in GCOL size / 2 vs color_count: {:X} vs {:X}",gcol_size/2,ret.color_count), LogLevel::Error);
        }
        let mut i: usize = 0;
        while i < ret.color_count as usize {
            match rdr.read_u16::<LittleEndian>() {
                Err(error) => {
                    log_write(format!("Error reading GCOL shorts: '{error}'"), LogLevel::Error);
                    return None;
                }
                Ok(cur_short_result) => ret.color_shorts.push(cur_short_result),
            };
            i += 1;
        }
        let final_position: usize = rdr.position() as usize;
        let segment_size: usize = bytedata.len();
        if final_position != segment_size {
            log_write(format!("GRAD: Mismatch in final position vs segment size: {:X} vs {:X}",final_position,segment_size), LogLevel::Error);
        }
        Some(ret)
    }
}
