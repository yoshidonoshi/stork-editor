// This file is for SCEN data segments within

use std::{fs, io::Cursor, path::PathBuf};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{engine::compression::{lamezip77_lz10_decomp, segment_wrap}, utils::{self, log_write, nitrofs_abs, LogLevel}};

use super::ScenSegment;

#[derive(Debug, Clone,PartialEq)]
pub struct ScenInfoData {
    pub layer_width: u16,
    pub layer_height: u16,
    /// To save space on hundreds of blank tiles, manually shove it down (check me)
    pub height_offset: u32,
    /// How fast the layer graphics move horizontally relative to Yoshi (0x1000 is matching)
    /// 
    /// Lower values move slower than Yoshi, higher values move faster than Yoshi
    pub x_scroll: u32,
    /// How fast the layer graphics move vertically relative to Yoshi (0x1000 is matching)
    /// 
    /// Lower values move slower than Yoshi, higher values move faster than Yoshi
    pub y_scroll: u32,
    /// This determines which actual "BG" in the engine it goes on
    pub which_bg: u8,
    pub layer_order: u8,
    pub char_base_block: u8,
    pub screen_base_block: u8,
    // See is_256_colorpal_mode
    /// 0x0 = 16 palette color, 0x1 = 256 palette color, 0x2 = 16 again, 0x3 = 256 again
    pub color_mode: u32,
    pub imbz_filename_noext: Option<String>
    // Don't forget to pad 4 bytes at the end?
}
impl Default for ScenInfoData {
    fn default() -> Self {
        Self {
            layer_width: 0x0000, layer_height: 0x0000,
            height_offset: 0xffff, x_scroll: 0x0000,
            y_scroll: 0x0000, which_bg: 0xff,
            layer_order: 0xff, char_base_block: 0xff,
            screen_base_block: 0xff, color_mode: 0xff,
            imbz_filename_noext: Option::None }
    }
}
impl ScenInfoData {
    pub fn new(rdr: &mut Cursor<&Vec<u8>>, internal_length: u32) -> Option<ScenInfoData> {
        // 24, 32, 36 are the only three sizes found with pytools
        if internal_length != 0x18 && internal_length != 0x20 && internal_length != 0x24 {
            log_write(format!("Unusual INFO size: 0x{:X}",internal_length), LogLevel::WARN);
        }
        let initial_position: u64 = rdr.position();
        let layer_width: u16 = utils::read_u16(rdr)?;
        let layer_height: u16 = utils::read_u16(rdr)?;
        let height_offset: u32 = utils::read_u32(rdr)?;
        let x_scroll: u32 = utils::read_u32(rdr)?;
        let y_scroll: u32 = utils::read_u32(rdr)?;
        let which_bg: u8 = utils::read_u8(rdr)?;
        let layer_order: u8 = utils::read_u8(rdr)?;
        let char_base_block: u8 = utils::read_u8(rdr)?;
        let screen_base_block: u8 = utils::read_u8(rdr)?;
        let color_mode: u32 = utils::read_u32(rdr)?;
        let mut imbz_filename_noext: Option<String> = Option::None;
        if internal_length > 0x18 {
            imbz_filename_noext = Some(utils::read_c_string(rdr));
        }
        let after_position: u64 = rdr.position();
        let mut read_length = after_position - initial_position;
        if read_length % 4 != 0 {
            log_write(format!("INFO read size not 4 byte aligned; size was 0x{:X}, padding",read_length), LogLevel::DEBUG);
            while read_length % 4 != 0 {
                let _ = rdr.read_u8();
                read_length = rdr.position() - initial_position;
            }
        }
        Some(ScenInfoData {
            layer_width,
            layer_height,
            height_offset,
            x_scroll,
            y_scroll,
            which_bg,
            layer_order,
            char_base_block,
            screen_base_block,
            color_mode,
            imbz_filename_noext
        })
    }

    pub fn get_imbz_pixels(&self, proj_dir: &PathBuf) -> Option<Vec<u8>> {
        let mut imbz_withext: String = self.imbz_filename_noext.clone().expect("imbz filename exists");
        imbz_withext.push_str(".imbz");
        let p: PathBuf = nitrofs_abs(proj_dir, &imbz_withext);
        let file_bytes = match fs::read(&p) {
            Err(error) => {
                log_write(format!("Failed to read IMBZ '{}' from INFO: '{error}'", p.display()), LogLevel::ERROR);
                return Option::None;
            }
            Ok(b) => b,
        };
        let pixels_decomped = lamezip77_lz10_decomp(&file_bytes);
        Some(pixels_decomped)
    }

    /// Returns true if the Colors/Palettes mode is 256, false if 16
    /// 
    /// Note: Here is one of the places it does & 1: 0202019a
    /// 
    /// Note: Ultimately the value is used here: 020128dc, but & 1 != 0, so 1 and 3 makes it 1, therefore 256
    /// 
    /// See: Colors/Palettes at https://www.problemkaputt.de/gbatek.htm#lcdiobgcontrol
    pub fn is_256_colorpal_mode(&self) -> bool {
        self.color_mode & 1 != 0
    }
}

impl ScenSegment for ScenInfoData {
    fn compile(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u16::<LittleEndian>(self.layer_width);
        let _ = comp.write_u16::<LittleEndian>(self.layer_height);
        let _ = comp.write_u32::<LittleEndian>(self.height_offset);
        let _ = comp.write_u32::<LittleEndian>(self.x_scroll);
        let _ = comp.write_u32::<LittleEndian>(self.y_scroll);
        let _ = comp.write_u8(self.which_bg);
        let _ = comp.write_u8(self.layer_order);
        let _ = comp.write_u8(self.char_base_block);
        let _ = comp.write_u8(self.screen_base_block);
        let _ = comp.write_u32::<LittleEndian>(self.color_mode);
        let Some(imbz_filename_noext) = &self.imbz_filename_noext else {
            // Already 4 padded, just return
            return comp;
        };

        // not Moving, so no need to clone
        let mut str_vec = imbz_filename_noext.bytes().collect();
        comp.append(&mut str_vec);
        comp.push(0x00); // Null terminator
        while comp.len() % 4 != 0 {
            comp.push(0x00);
        }

        let final_comp_len = comp.len();
        // 24, 32, 36 are the only ones found in the base game
        if final_comp_len != 0x18 && final_comp_len != 0x20 && final_comp_len != 0x24 {
            log_write(format!("Unusual INFO compiled size: 0x{:X}",final_comp_len), LogLevel::ERROR);
        }

        comp
    }

    fn wrap(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        let comped = self.compile(Option::None);
        segment_wrap(&comped, self.header())
    }

    fn header(&self) -> String {
        String::from("INFO")
    }
}
