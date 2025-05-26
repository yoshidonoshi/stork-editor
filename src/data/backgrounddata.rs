// This represents the **SCEN** data inside MPDZ files,
//   which in turns has data for a background
// It contains uncompressed versions of things for
//   fast reading by the Gui engine
// Saving will require recompiling it and saving it
//   back on top of the segment inside MapData

use std::{fmt, io::{Cursor, Read}, path::PathBuf};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{engine::compression::{lamezip77_lz10_decomp, segment_wrap}, utils::{header_to_string, log_write, LogLevel}};

use super::{scendata::{anmz::AnmzDataSegment, colz::CollisionData, imbz::ImbzData, imgb::ImgbData, info::ScenInfoData, mpbz::MapTileDataSegment, plan::AnimatedPaletteData, pltb::PltbData, rast::RastData, scrl::ScrollData, ScenSegment, ScenSegmentWrapper}, types::Palette, TopLevelSegment};

#[derive(Debug,Clone,PartialEq,Default)]
pub struct BackgroundData {
    /// TODO: Get rid of this, only left in constructor
    pub info_ro: ScenInfoData,
    /// This is used to offset map tile palette values during render
    pub _pal_offset: u8,
    /// Unedited, straight out of the data. Cache it once rendered
    pub pixel_tiles_preview: Option<Vec<u8>>, // For previews
    pub scen_segments: Vec<ScenSegmentWrapper>,
}
impl fmt::Display for BackgroundData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"BackgroundData [ segments.len={}, ]",self.scen_segments.len())
    }
}
impl BackgroundData {
    pub fn new(vec: &Vec<u8>, project_directory: &PathBuf) -> Result<BackgroundData,String> {
        // Since the issue is commonly tied to a specific background, this should stick out
        log_write("> Creating SCEN...", LogLevel::DEBUG);
        let mut ret: BackgroundData = BackgroundData::default();
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(vec);
        let file_end_pos: u64 = vec.len().try_into().unwrap();
        let mut test_load_count: usize = 0;
        while rdr.position() < file_end_pos {
            test_load_count += 1;
            // Data for loading loop    
            let seg_header = rdr.read_u32::<LittleEndian>().unwrap();
            let seg_internal_length = rdr.read_u32::<LittleEndian>().unwrap();
            let seg_header_str = header_to_string(&seg_header);
            log_write(format!("Reading sub-segment '{}' with size 0x{:X}",seg_header_str,seg_internal_length), LogLevel::DEBUG);

            match seg_header_str.as_str() {
                "INFO" => {
                    let info = match ScenInfoData::new(&mut rdr, seg_internal_length) {
                        Some(i) => i,
                        None => {
                            return Err("Failed to create INFO".to_owned());
                        }
                    };
                    ret.scen_segments.push(ScenSegmentWrapper::INFO(info.clone()));
                    // Is there IMBZ data to retrieve?
                    if info.imbz_filename_noext.is_some() {
                        // There is IMBZ data to retrieve. Fetch!
                        let pixels_decomped = info.get_imbz_pixels(project_directory);
                        if pixels_decomped.is_none() {
                            // It already checked if it exists. If not, MAJOR error
                            log_write(format!("Failed to get IMBZ from INFO on BG layer {}",info.which_bg), LogLevel::ERROR);
                            continue;
                        }
                        let pixels_decomped = pixels_decomped.unwrap();
                        ret.pixel_tiles_preview = Some(pixels_decomped);
                    }
                    ret.info_ro = info;
                }
                "COLZ" => {
                    let mut compressed_buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    let _read_res = rdr.read_exact(&mut compressed_buffer);
                    let colz_obj = CollisionData::new(&compressed_buffer);
                    ret.scen_segments.push(ScenSegmentWrapper::COLZ(colz_obj.clone()));
                }
                "PLTB" => {
                    let mut pal_vec: Vec<Palette> = Vec::new();
                    if ret.info_ro.color_mode > 0x1 {
                        log_write(format!("Warning: PLTB color mode {} may be poorly supported",ret.info_ro.color_mode), LogLevel::WARN);
                    }
                    if !ret.info_ro.is_256_colorpal_mode() {
                        log_write("Loading PLTB with 16 color format", LogLevel::DEBUG);
                        // Palette in 16 mode: each is 16 colors * 2 bytes
                        let count_16: u32 = seg_internal_length / (16*2);
                        let mut index: u32 = 0;
                        while index < count_16 {
                            let pal = Palette::from_cur(&mut rdr,16);
                            pal_vec.push(pal);
                            index += 1;
                        }
                    } else {
                        log_write("Loading PLTB with 256 color format", LogLevel::DEBUG);
                        // Note: Not every 256 palette has 256 colors
                        // Read forwards, including garbage data (probably don't do this)
                        // But the garbage data will never be read anyway so...
                        // Future issue may be if this is the last segment with nothing after it
                        let start_pos = rdr.position();
                        pal_vec.push(Palette::from_cur(&mut rdr, 256));
                        rdr.set_position(start_pos + seg_internal_length as u64);
                    }
                    let pltb = PltbData::from_pal_vec(pal_vec);
                    let pltb_wrapped = ScenSegmentWrapper::PLTB(pltb.clone());
                    ret.scen_segments.push(pltb_wrapped);
                }
                "MPBZ" => {
                    let mut buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    let _read_res = rdr.read_exact(&mut buffer);
                    let mp_decomp = lamezip77_lz10_decomp(&buffer);
                    let mpbz = MapTileDataSegment::from_decomped_vec(&mp_decomp,ret.info_ro.layer_width);
                    // Probably get rid of this eventually, or only activate in debug mode
                    mpbz.test_against_raw_decomp(&Some(ret.info_ro.clone()), &mp_decomp);
                    let mpbz_wrapped = ScenSegmentWrapper::MPBZ(mpbz.clone());
                    ret.scen_segments.push(mpbz_wrapped);
                }
                "IMGB" => {
                    let mut buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    let _read_res = rdr.read_exact(&mut buffer);
                    let imgb_data = ImgbData::new(&buffer);
                    ret.scen_segments.push(ScenSegmentWrapper::IMGB(imgb_data));
                    // Update preview
                    if ret.pixel_tiles_preview.is_some() {
                        log_write("IMGB: Attempting to write to pixeltiles when already contains data", LogLevel::WARN);
                    }
                    ret.pixel_tiles_preview = Some(buffer.clone()); // Hand the actual data into it
                }
                "IMBZ" => {
                    let mut imbz_comped_buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    let _read_res = rdr.read_exact(&mut imbz_comped_buffer);
                    let wrapped = ScenSegmentWrapper::IMBZ(ImbzData::new(&imbz_comped_buffer));
                    ret.scen_segments.push(wrapped);

                    // Now decompress it for the preview
                    let imbz_decomped = lamezip77_lz10_decomp(&imbz_comped_buffer);
                    if ret.pixel_tiles_preview.is_some() {
                        log_write("IMBZ: Attempting to write to pixeltiles when already contains data", LogLevel::WARN);
                    }
                    ret.pixel_tiles_preview = Some(imbz_decomped); // Move it in
                }
                "ANMZ" => {
                    let mut buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    let _read_res = rdr.read_exact(&mut buffer);
                    let anmz_decomped = lamezip77_lz10_decomp(&buffer);
                    // The real one to use for previews
                    let anmz_data = AnmzDataSegment::from_decomp(&anmz_decomped);
                    ret.scen_segments.push(ScenSegmentWrapper::ANMZ(anmz_data.clone()));
                }
                "SCRL" => {
                    let scrl = ScrollData::new(&mut rdr);
                    let scrl_seg = ScenSegmentWrapper::SCRL(scrl);
                    ret.scen_segments.push(scrl_seg);
                }
                "PLAN" => {
                    let mut buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    let _read_res = rdr.read_exact(&mut buffer);
                    let plan = AnimatedPaletteData::new(&buffer);
                    ret.scen_segments.push(ScenSegmentWrapper::PLAN(plan));
                }
                "RAST" => {
                    let mut buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    let _read_res = rdr.read_exact(&mut buffer);
                    let rast = RastData::new(&buffer);
                    ret.scen_segments.push(ScenSegmentWrapper::RAST(rast));
                }
                _ => {
                    // I wrote a script to check every single one
                    // This should not be possible
                    let unknown_seg = format!("Unknown segment in SCEN: '{}'",&seg_header_str);
                    log_write(unknown_seg.clone(), LogLevel::ERROR);
                    return Err(unknown_seg);
                    // let mut _buffer: Vec<u8> = vec![0;seg_internal_length as usize];
                    // let _read_res = rdr.read_exact(&mut _buffer);
                }
            }
        }

        // Apply ANMZ preview //
        if let Some(anmz_data) = ret.clone().get_anmz() {
            let mut cur_vram_offset: usize = anmz_data.vram_offset as usize;
            if ret.info_ro.color_mode > 0x1 {
                log_write("Color Modes above 1 may be poorly supported", LogLevel::WARN);
            }
            if ret.info_ro.is_256_colorpal_mode() {
                cur_vram_offset *= 64;
            } else {
                cur_vram_offset *= 32;
            }
            if let Some(pixeltiles) = &mut ret.pixel_tiles_preview {
                for pixeltile in &anmz_data.pixeltiles {
                    // This could probably be done more efficiently
                    while cur_vram_offset >= pixeltiles.len() {
                        pixeltiles.push(0x00);
                    }
                    pixeltiles[cur_vram_offset] = *pixeltile;
                    cur_vram_offset += 1;
                }
            } else {
                log_write("Unable to unwrap pixeltiles when creating ANMZ", LogLevel::ERROR);
            }
        }

        if ret.scen_segments.len() != test_load_count {
            let mismatch_msg = format!("Mismatch in loaded segments versus load count: {} vs {}",
                ret.scen_segments.len(),test_load_count);
            log_write(mismatch_msg.clone(), LogLevel::ERROR);
            return Err(mismatch_msg);
        }

        log_write(format!("> Created SCEN for background {}",ret.info_ro.which_bg), LogLevel::DEBUG);

        Ok(ret)
    }

    pub fn get_mpbz_mut(&mut self) -> Option<&mut MapTileDataSegment> {
        for seg in &mut self.scen_segments {
            if let ScenSegmentWrapper::MPBZ(mpbz) = seg {
                return Some(mpbz);
            }
        }
        Option::None
    }

    pub fn get_mpbz(&self) -> Option<&MapTileDataSegment> {
        for seg in &self.scen_segments {
            if let ScenSegmentWrapper::MPBZ(mpbz) = seg {
                return Some(mpbz);
            }
        }
        Option::None
    }

    pub fn get_colz_mut(&mut self) -> Option<&mut CollisionData> {
        for seg in &mut self.scen_segments {
            if let ScenSegmentWrapper::COLZ(colz) = seg {
                return Some(colz);
            }
        }
        Option::None
    }

    pub fn get_colz(&self) -> Option<&CollisionData> {
        for seg in &self.scen_segments {
            if let ScenSegmentWrapper::COLZ(colz) = seg {
                return Some(colz);
            }
        }
        Option::None
    }

    pub fn get_pltb_mut(&mut self) -> Option<&mut PltbData> {
        for seg in &mut self.scen_segments {
            if let ScenSegmentWrapper::PLTB(pltb) = seg {
                return Some(pltb);
            }
        }
        Option::None
    }

    pub fn get_pltb(&self) -> Option<&PltbData> {
        for seg in &self.scen_segments {
            if let ScenSegmentWrapper::PLTB(pltb) = seg {
                return Some(pltb);
            }
        }
        Option::None
    }

    pub fn get_anmz(&self) -> Option<&AnmzDataSegment> {
        for seg in &self.scen_segments {
            if let ScenSegmentWrapper::ANMZ(anmz) = seg {
                return Some(anmz);
            }
        }
        Option::None
    }

    pub fn get_info(&self) -> Option<&ScenInfoData> {
        for seg in &self.scen_segments {
            if let ScenSegmentWrapper::INFO(info) = seg {
                return Some(info);
            }
        }
        Option::None
    }

    pub fn get_info_mut(&mut self) -> Option<&mut ScenInfoData> {
        for seg in &mut self.scen_segments {
            if let ScenSegmentWrapper::INFO(info) = seg {
                return Some(info);
            }
        }
        Option::None
    }

    pub fn increase_width(&mut self, new_width: u16) -> Result<u16,()> {
        if new_width % 2 != 0 {
            log_write(format!("Cannot make width odd (0x{:X})",new_width),LogLevel::WARN);
            return Err(());
        }
        log_write(format!("Changing width of layer to 0x{:X}",new_width),LogLevel::LOG);
        let info_c = self.get_info().expect("INFO is always there");
        let old_width = info_c.layer_width;
        if new_width <= old_width {
            log_write(format!("Cannot increase, new width vs old: {:X} vs {:X}",new_width,old_width), LogLevel::ERROR);
            return Err(());
        }
        let how_much_add = new_width - old_width;
        if let Some(mpbz) = self.get_mpbz_mut() {
            mpbz.increase_width(old_width, how_much_add as usize);
        }
        if let Some(colz) = self.get_colz_mut() {
            colz.increase_width(old_width, how_much_add as usize);
        }
        let info = self.get_info_mut().expect("Done earlier");
        info.layer_width = new_width;
        Ok(info.layer_width)
    }

    pub fn decrease_width(&mut self, new_width: u16) -> Result<u16,()> {
        if new_width % 2 != 0 {
            log_write(format!("Cannot make width odd (0x{:X})",new_width),LogLevel::WARN);
            return Err(());
        }
        log_write(format!("Changing width of layer to 0x{:X}",new_width),LogLevel::LOG);
        let info_c = self.get_info().expect("INFO is always there");
        let old_width = info_c.layer_width;
        if new_width >= old_width {
            log_write(format!("Cannot decrease, new width vs old: {:X} vs {:X}",new_width,old_width), LogLevel::ERROR);
            return Err(());
        }
        let how_much_remove = old_width - new_width;
        if let Some(mpbz) = self.get_mpbz_mut() {
            mpbz.decrease_width(old_width, how_much_remove as usize);
        }
        if let Some(colz) = self.get_colz_mut() {
            colz.decrease_width(old_width as i32, how_much_remove as i32);
        }
        let info = self.get_info_mut().expect("Done earlier");
        info.layer_width = new_width;
        Ok(info.layer_width)
    }

    pub fn change_height(&mut self, new_height: u16) -> Result<u16,()> {
        let info_c = self.get_info().expect("INFO is always there").clone();
        if new_height % 2 != 0 {
            log_write(format!("Cannot make height odd (0x{:X})",new_height),LogLevel::WARN);
            return Err(());
        }
        if let Some(mpbz) = self.get_mpbz_mut() {
            mpbz.change_height(new_height, info_c.layer_width);
        }
        if let Some(colz) = self.get_colz_mut() {
            colz.change_height(new_height, info_c.layer_width);
        }
        let info = self.get_info_mut().expect("Done earlier");
        info.layer_height = new_height;
        Ok(info.layer_height)
    }
}

impl TopLevelSegment for BackgroundData {
    fn compile(&self) -> Vec<u8> {
        let mut compiled: Vec<u8> = Vec::new();
        let info_c = self.get_info().expect("There is always INFO");
        for segment in &self.scen_segments {
            let mut seg_comp = segment.wrap(&Some(info_c.clone()));
            compiled.append(&mut seg_comp);
        }

        compiled
    }
    
    fn wrap(&self) -> Vec<u8> {
        let uncomped_bytes: Vec<u8> = self.compile();
        // SCEN files are not compressed, though sub-segments are
        segment_wrap(&uncomped_bytes, "SCEN".to_owned())
    }

    fn header(&self) -> String {
        String::from("SCEN")
    }
}
