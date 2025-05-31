use std::{collections::HashMap, env, f32::consts::PI, fs::{self, write}, io::{Cursor, Read}, path::PathBuf};

use byteorder::{LittleEndian, ReadBytesExt};
use colored::Colorize;
use egui::{pos2, Color32, ColorImage, Pos2, Rect, TextureHandle};

use crate::{data::{path::PathPoint, types::{MapTileRecordData, Palette}}, gui::windows::paths_win::PathAngle};

pub mod profile;

#[derive(PartialEq)]
pub enum LogLevel {
    DEBUG,
    LOG,
    WARN,
    ERROR,
    FATAL
}

pub fn log_write(msg: impl Into<String>, level: LogLevel) {
    let msg = msg.into();
    match level {
        LogLevel::DEBUG => {
            if !is_debug() {
                return;
            }
            println!("[DEBUG] {msg}");
            log::debug!("{msg}");
        }
        LogLevel::LOG => {
            println!("[{}] {msg}","INFO".green());
            log::info!("{msg}");
        }
        LogLevel::WARN => {
            println!("[{}] {msg}","WARN".yellow());
            log::warn!("{msg}");
        }
        LogLevel::ERROR => {
            println!("[{}] {msg}","ERROR".red());
            log::error!("{msg}");
        }
        LogLevel::FATAL => {
            println!("[{}] {msg}","FATAL".red());
            log::error!("{msg}");
            panic!("{}",msg);
        }
    }
}

#[allow(dead_code)] // May not be used in final
pub fn print_vector_u8(byte_vector: &Vec<u8>) {
    let vec_length: usize = byte_vector.len();
    if vec_length == 0 {
        log_write("print_vector_u8: vector is empty", LogLevel::LOG);
    }
    let mut i: usize = 0;
    while i < vec_length {
        let mut end: usize = i+0x10;
        if end > byte_vector.len() {
            end = byte_vector.len();
        }
        let hex_line: String = byte_vector[i..end].iter().map(|b| format!("{:02X} ",b)).collect();
        let starting_string = format!("0x{:05X}",i);
        println!("{starting_string} | {hex_line}");
        i += 0x10;
        if i > 0xffffff {
            log_write("i index too high in print_vector_u8!", LogLevel::LOG);
            break;
        }
    }
}

pub fn get_sin_cos_table_value(arm9: &Vec<u8>, value: u16) -> PathAngle {
    const TABLE_ADDR: u32 = 0x0d1878; //0x020d1878;
    let mut rdr: Cursor<&Vec<u8>> = Cursor::new(arm9);
    // Value 1
    let pos1 = TABLE_ADDR + ((value as u32 >> 4) * 2 + 1) * 2;
    rdr.set_position(pos1 as u64);
    let sh1 = rdr.read_i16::<LittleEndian>().expect("Reading SinCos value 1");
    // Value 2
    let pos2 = TABLE_ADDR + ((value as u32 >> 4) * 2 + 0) * 2;
    rdr.set_position(pos2 as u64);
    let sh2 = rdr.read_i16::<LittleEndian>().expect("Reading SinCos value 2");
    PathAngle { x: sh1, y: sh2 }
}

#[allow(dead_code)] // May not be used in final
pub fn compare_vector_u8s(byte_vector_1: &Vec<u8>, byte_vector_2: &Vec<u8>) {
    if byte_vector_1.len() != byte_vector_2.len() {
        log_write(format!("Vector lengths differ: 0x{:X} vs 0x{:X}",byte_vector_1.len(),byte_vector_2.len()),LogLevel::ERROR);
        return;
    }
    let mut index = 0;
    let both_length = byte_vector_1.len();
    while index < both_length {
        let value_1 = byte_vector_1[index];
        let value_2 = byte_vector_2[index];
        if value_1 != value_2 {
            log_write(format!("Value mismatch at index 0x{:X}: 0x{:X} vs 0x{:X}",index,value_1,value_2),LogLevel::ERROR);
            return;
        }
        index += 1;
    }
    log_write(format!("Vectors with length 0x{:X} match!", both_length),LogLevel::DEBUG);
}

pub fn header_to_string(header: &u32) -> String {
    let char0 = (header >> 24) % 0x100;
    let char1 = (header >> 16) % 0x100;
    let char2 = (header >> 8) % 0x100;
    let char3 = (header >> 0) % 0x100;
    let char0 = std::char::from_u32(char0).unwrap_or('�');
    let char1 = std::char::from_u32(char1).unwrap_or('�');
    let char2 = std::char::from_u32(char2).unwrap_or('�');
    let char3 = std::char::from_u32(char3).unwrap_or('�');
    let str = format!("{char3}{char2}{char1}{char0}");
    str
}

pub fn settings_to_string(settings: &Vec<u8>) -> String {
    settings.iter().map(|f| {
        format!("{:02X} ",f)
    }).collect::<String>().trim().to_string()
}

pub fn string_to_settings(settings_string: &String) -> Result<Vec<u8>,String> {
    let split: Vec<&str> = settings_string.trim().split(' ').collect();
    let mut new_settings: Vec<u8> = Vec::new();
    for str8 in split {
        match u8::from_str_radix(str8, 16) {
            Ok(u8val) => new_settings.push(u8val),
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(new_settings)
}

pub fn get_curve_fine(cur_point: &PathPoint, next_point: &PathPoint) -> (Pos2,i32,f32) {
    const RAD_UNIT: f32 = PI / 2.0;
    let rads: f32;
    let is_next_above = next_point.y_fine < cur_point.y_fine;
    let is_next_rightwards = next_point.x_fine > cur_point.x_fine;
    let is_turning_right = cur_point.angle >= 0;
    let mut circle_point_fine: Pos2 = Pos2::ZERO;
    // Yes, it's this weirdly complex in the source code too...
    if is_turning_right { // Inverted if Bezier
        if is_next_above && is_next_rightwards {
            // Up and to the right
            circle_point_fine.x = next_point.x_fine as f32;
            circle_point_fine.y = cur_point.y_fine as f32;
            rads = RAD_UNIT * 1.0; // top left curve
        } else if is_next_above && !is_next_rightwards {
            // Up and to the left
            circle_point_fine.x = cur_point.x_fine as f32;
            circle_point_fine.y = next_point.y_fine as f32;
            rads = RAD_UNIT * 2.0; // ?
        } else if !is_next_above && is_next_rightwards {
            // Below and to the right
            circle_point_fine.x = cur_point.x_fine as f32;
            circle_point_fine.y = next_point.y_fine as f32;
            rads = RAD_UNIT * 0.0;
        } else { // !is_next_above && !is_next_rightwards
            // Below and to the left
            circle_point_fine.x = next_point.x_fine as f32;
            circle_point_fine.y = cur_point.y_fine as f32;
            rads = RAD_UNIT * 3.0; // bottom right
        }
    } else { // Turning left
        if is_next_above && is_next_rightwards {
            // Up and to the right
            circle_point_fine.x = cur_point.x_fine as f32;
            circle_point_fine.y = next_point.y_fine as f32;
            rads = RAD_UNIT * 3.0; // bottom right
        } else if is_next_above && !is_next_rightwards {
            // Up and to the left
            circle_point_fine.x = next_point.x_fine as f32;
            circle_point_fine.y = cur_point.y_fine as f32;
            rads = RAD_UNIT * 0.0; // Top right
        } else if !is_next_above && is_next_rightwards {
            // Below and to the right
            circle_point_fine.x = next_point.x_fine as f32;
            circle_point_fine.y = cur_point.y_fine as f32;
            rads = RAD_UNIT * 2.0; // Bottom left
        } else {
            // Below and to the left
            circle_point_fine.x = cur_point.x_fine as f32;
            circle_point_fine.y = next_point.y_fine as f32;
            rads = RAD_UNIT * 1.0; // Top left
        }
    }
    let radius_fine: i32 = (cur_point.y_fine as i32) - (next_point.y_fine as i32);
    (circle_point_fine,radius_fine.abs(),rads)
}

pub fn string_to_header(header: String) -> u32 {
    if header.len() != 4 {
        log_write(format!("string_to_header should intake 4 character String, not {}",header.len()), LogLevel::ERROR);
    }
    let header_bytes: &[u8] = header.as_bytes();
    let header_vec = Vec::from(header_bytes);
    let mut rdr: Cursor<&Vec<u8>> = Cursor::new(&header_vec);
    match rdr.read_u32::<LittleEndian>() {
        Err(error) => {
            log_write(format!("Failed to read u32 in string_to_header: {}", error), LogLevel::ERROR);
            return 0xFFFFFFFF;
        },
        Ok(read_res) => read_res,
    }
}

pub fn color_from_u16(val: &u16) -> Color32 {
    let red: u16 = val & 0b000000000011111;
    let green: u16 = (val & 0b000001111100000) >> 5;
    let blue: u16 = (val & 0b111110000000000) >> 10;
    let red = (red as f32) * 8.2;
    let green = (green as f32) * 8.2;
    let blue = (blue as f32) * 8.2;
    let color = Color32::from_rgb(red as u8, green as u8, blue as u8);
    color
}

pub fn read_c_string(rdr: &mut Cursor<&Vec<u8>>) -> String {
    // Read the map file name
    let mut string_buffer: Vec<u8> = Vec::new();
    while let Ok(charbyte) = rdr.read_u8() {
        if charbyte == 0x00 {
            break;
        }
        string_buffer.push(charbyte);
    }
    match String::from_utf8(string_buffer) {
        Err(_) => {
            log_write("Failed to read mpdz_name_noext", LogLevel::FATAL);
            String::new() // Satisfy compiler, but FATAL panics with message
        }
        Ok(s) => s,
    }
}

pub fn read_address(rdr: &mut Cursor<&Vec<u8>>) -> Option<u32> {
    let mut address: u32 = read_u32(rdr)?;
    address -= 0x2000000;
    Some(address)
}

pub fn read_fixed_string(vec_data: &Vec<u8>, position: u64, length: u32) -> String {
    let mut rdr: Cursor<&Vec<u8>> = Cursor::new(vec_data);
    rdr.set_position(position);
    read_fixed_string_cursor(&mut rdr, length)
}

pub fn read_fixed_string_cursor(rdr: &mut Cursor<&Vec<u8>>, length: u32) -> String {
    let mut string_buffer: Vec<u8> = Vec::new();
    let mut i: u32 = 0;
    while i < length {
        match rdr.read_u8() {
            Err(error) => {
                log_write(format!("char_byte read error: '{}'", error), LogLevel::ERROR);
                return "READERROR".to_owned();
            }
            Ok(char_byte) => string_buffer.push(char_byte),
        }
        i += 1;
    }
    match String::from_utf8(string_buffer) {
        Err(_) => {
            log_write("Failed to read fixed string", LogLevel::FATAL);
            String::new() // Satisfy compiler, but FATAL panics with message
        }
        Ok(result_string) => result_string,
    }
}

pub fn color_image_from_pal(pal: &Palette, pal_indexes: &Vec<u8>) -> ColorImage {
    let mut ret: Vec<egui::Color32> = Vec::new();
    if pal_indexes.len() != 64 {
        log_write(format!("Instead of 64 values when generating color image, got {}, placing red error tile",pal_indexes.len()), LogLevel::ERROR);
        return egui::ColorImage {
            size: [8,8],
            pixels: vec![Color32::RED;64]
        };
    }
    for n in pal_indexes {
        if *n == 0 {
            let col32: Color32 = Color32::TRANSPARENT;
            ret.push(col32);
        } else {
            let col32: Color32 = pal.colors[*n as usize].color.clone();
            ret.push(col32);
        }
    }
    let color_image: ColorImage = egui::ColorImage {
        size: [8,8],
        pixels: ret
    };
    color_image
}

pub fn generate_bg_tile_cache(ctx: &egui::Context, color_images: Vec<ColorImage>) -> Vec<TextureHandle> {
    let mut ret: Vec<TextureHandle> = Vec::new();
    for ci in color_images {
        let tex_handle = ctx.load_texture("tile", ci, egui::TextureOptions::NEAREST);
        // let size = tex_handle.size_vec2();
        // let sized_image = egui::load::SizedTexture::new(tex_handle.id(), size);
        // let image: Image<'_> = egui::Image::from_texture(sized_image);
        ret.push(tex_handle);
    }
    ret
}

pub fn pixel_byte_array_to_nibbles(byte_array: &Vec<u8>) -> Vec<u8> {
    if byte_array.len() != 0x20 {
        log_write(format!("byte_array in pixel_byte_array_to_nibbles was not 32, was instead {}",byte_array.len()), LogLevel::ERROR);
    }
    let mut ret: Vec<u8> = Vec::new();
    for byte in byte_array {
        let lower_bits = byte % 0x10;
        ret.push(lower_bits);

        let high_bits = byte >> 4;
        ret.push(high_bits);
    }
    if ret.len() != 64 {
        log_write(format!("ret in pixel_byte_array_to_nibbles was not 64, was instead {}",ret.len()), LogLevel::ERROR);
    }
    ret
}

#[allow(dead_code)] // May not be used in final
pub fn print_cursor(rdr: &mut Cursor<&Vec<u8>>, length: usize) {
    let base_position = rdr.position();
    let mut buffer: Vec<u8> = vec![0;length];
    let _read_res = rdr.read_exact(&mut buffer);
    print_vector_u8(&buffer);
    rdr.set_position(base_position);
}

#[allow(dead_code)] // May not be used in final
pub fn write_vec_test_file(byte_vector: &Vec<u8>,filename: String) {
    let result = write(&filename, byte_vector);
    if result.is_err() {
        log_write(format!("Failed to write vec test file '{}'",&filename), LogLevel::ERROR);
    }
}

pub fn nitrofs_abs(export_dir: &PathBuf,filename_local: &String) -> PathBuf {
    let mut p: PathBuf = export_dir.clone();
    p.push("files");
    p.push("file");
    p.push(filename_local);
    p
}

pub fn get_backup_folder(export_dir: &PathBuf) -> Result<PathBuf,()> {
    let mut p: PathBuf = PathBuf::from(export_dir);
    p.push("backups");
    if !p.exists() {
        let create_res = fs::create_dir(p.clone());
        if create_res.is_err() {
            log_write(format!("Error creating backup folder: '{}'",create_res.unwrap_err()), LogLevel::ERROR);
            return Err(());
        }
    }
    Ok(p)
}

pub fn get_template_folder(export_dir: &PathBuf) -> Option<PathBuf> {
    let mut p: PathBuf = PathBuf::from(export_dir);
    p.push("templates");
    if !p.exists() {
        let create_res = fs::create_dir(p.clone());
        if create_res.is_err() {
            log_write(format!("Error creating template folder: '{}'",create_res.unwrap_err()), LogLevel::ERROR);
            return None;
        }
    }
    Some(p)
}

pub fn get_map_templates() -> HashMap<String,String> {
    HashMap::from([
        ("Flower Garden - Full".to_string(), "01k3380.mpdz".to_string()),
        ("Spawn Pipe Interior".to_string(), "15k0382.mpdz".to_string()),
        ("Generic Pipe Interior".to_string(), "15k0383.mpdz".to_string()),
        ("Cave - Godrays".to_string(), "08k3381.mpdz".to_string()),
        ("Cave - Mushrooms".to_string(), "08k5371.mpdz".to_string()),
        ("Cave - Wall Holes".to_string(), "08y0213.mpdz".to_string()),
        ("Cave - Lava Goal".to_string(), "08k0353.mpdz".to_string()),
        ("Cave - Lava Vines".to_string(), "08i0045.mpdz".to_string()),
        ("Cliff Tunnels - Flowers".to_string(), "09k3117.mpdz".to_string()),
        ("Cliff Tunnels - Soft Rock".to_string(), "09k5120.mpdz".to_string()),
        ("Cliff Tunnels - Outside".to_string(), "09k5121.mpdz".to_string()),
        ("Cliff Tunnels - Top".to_string(), "09k3243.mpdz".to_string()),
        ("Fortress - Lava".to_string(), "14k5361.mpdz".to_string()),
        ("Fortress - Boss Room".to_string(), "14m0006.mpdz".to_string()),
        ("Fortress - Ship Interior".to_string(), "14w2006.mpdz".to_string()),
        ("Fortress - Metal Spikes".to_string(), "14w2000.mpdz".to_string()),
        ("Jungle - Vines".to_string(), "16m0046.mpdz".to_string()),
        ("Jungle - Goal".to_string(), "16m0052.mpdz".to_string()),
        ("Jungle - Soft Rock".to_string(), "16m0079.mpdz".to_string()),
        ("Castle Roof - Spike Ferries".to_string(), "13w0113.mpdz".to_string()),
        ("Castle Roof - Soft Rock".to_string(), "13w0111.mpdz".to_string()),
        ("Castle Roof - Minigame 1".to_string(), "13y0920.mpdz".to_string()),
        ("Castle Roof - Log Platforms".to_string(), "13w0112.mpdz".to_string()),
        ("Castle Roof - Interior".to_string(), "13w0110.mpdz".to_string()),
        ("Jungle River - Teeter Totters".to_string(), "04w0391.mpdz".to_string()),
        ("Jungle River - Waterfall".to_string(), "04w0394.mpdz".to_string()),
        ("Jungle River - Goal".to_string(), "04w0395.mpdz".to_string()),
        ("High Seas - Rainy Pirate Ship".to_string(), "05k0430.mpdz".to_string()),
        ("High Seas - Mud Islands".to_string(), "05k3421.mpdz".to_string()),
        ("High Seas - Night Ship".to_string(), "05k3425.mpdz".to_string()),
        ("Clouds - Moving Platforms".to_string(), "11w0314.mpdz".to_string()),
        ("Clouds - Goal".to_string(), "11w0317.mpdz".to_string()),
        ("Clouds - Large + Spikes".to_string(), "11y0620.mpdz".to_string()),
        ("Clouds - Soft Rock".to_string(), "11i0047.mpdz".to_string()),
        ("Sky Stones - Stones".to_string(), "03k4001.mpdz".to_string()),
        ("Sky Stones - Grass".to_string(), "03k4003.mpdz".to_string()),
        ("Sky Stones - Wall Holes".to_string(), "03k4004.mpdz".to_string()),
        ("Sky Stones - Goal".to_string(), "03i0006.mpdz".to_string()),
        ("Outback - Short Caves".to_string(), "02k0112.mpdz".to_string()),
        ("Outback - Caves".to_string(), "02k0250.mpdz".to_string()),
        ("Outback - Goal".to_string(), "02k0113.mpdz".to_string()),
        ("Outback - Rails".to_string(), "02w0039.mpdz".to_string()),
        ("Space - Asteroids".to_string(), "17w0114.mpdz".to_string()),
        ("Snow - Ice".to_string(), "10k7101.mpdz".to_string()),
        ("Snow - Trees".to_string(), "10k5102.mpdz".to_string()),
        ("Snow - Goal".to_string(), "10k7104.mpdz".to_string()),
        ("Snow - Moving Platforms".to_string(), "10k5012.mpdz".to_string()),
        ("Snow - Skiing".to_string(), "10k6008.mpdz".to_string())
    ])
}

/// Get the Rect determining how the tile is flipped
pub fn get_uvs_from_tile(tile: &MapTileRecordData) -> Rect {
    let mut uvs = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    if tile.flip_h && !tile.flip_v {
        uvs = Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0));
    } else if !tile.flip_h && tile.flip_v {
        uvs = Rect::from_min_max(pos2(0.0, 1.0), pos2(1.0, 0.0));
    } else if tile.flip_h && tile.flip_v {
        uvs = Rect::from_min_max(pos2(1.0, 1.0), pos2(0.0, 0.0));
    }
    uvs
}

pub fn get_pixel_bytes_16(pixel_tiles: &Vec<u8>, tile_id: &u16) -> Vec<u8> {
    let array_start: usize = *tile_id as usize * 32;
    let array_end: usize = array_start + 32;
    if array_end > pixel_tiles.len() {
        // Without ANMZ, this fired constantly
        log_write(format!("get_pixel_bytes_16 draw overflow, offending tile_id: 0x{:X}/{}",tile_id,tile_id), LogLevel::ERROR);
        return [1;64].to_vec();
    }
    let byte_array = pixel_tiles[array_start..array_end].to_vec();
    byte_array
}

pub fn get_pixel_bytes_256(pixel_tiles: &Vec<u8>, tile_id: &u16) -> Vec<u8> {
    let array_start: usize = *tile_id as usize * 64;
    let array_end: usize = array_start + 64;
    if array_end > pixel_tiles.len() {
        // Without ANMZ, this fired constantly
        log_write(format!("get_pixel_bytes_256 draw overflow(0x{:X} >= 0x{:X}), offending tile_id: 0x{:X}/{}",
            array_end,pixel_tiles.len(),tile_id,tile_id), LogLevel::ERROR);
        return [16;64].to_vec();
    }
    let byte_array = pixel_tiles[array_start..array_end].to_vec();
    byte_array
}

pub fn get_x_pos_of_map_index(map_index: u32, map_width: &u32) -> u16 {
    //println!("get_x_pos_of_map_index: {},{} => {}",&map_index,&map_width,map_index % map_width);
    let res = map_index % map_width;
    if res > u16::MAX as u32 {
        log_write(format!("get_x_pos_of_map_index too high: {} > u16::MAX({})",res,u16::MAX), LogLevel::ERROR);
        return 0;
    }
    res as u16
}

pub fn get_y_pos_of_map_index(map_index: u32, map_width: &u32) -> u16 {
    let res = map_index / map_width;
    if res > u16::MAX as u32 {
        log_write(format!("get_y_pos_of_map_index too high: {} > u16::MAX({})",res,u16::MAX), LogLevel::ERROR);
        return 0;
    }
    res as u16
}

pub fn xy_to_index(x_index: u32, y_index: u32, map_width: &u32) -> u32 {
    x_index + (*map_width * y_index)
}

pub fn distance(p1: Pos2, p2: Pos2) -> f32 {
    (p2.x - p1.x).hypot(p2.y - p1.y)
}

pub fn read_u8(rdr: &mut Cursor<&Vec<u8>>) -> Option<u8> {
    match rdr.read_u8() {
        Ok(i) => Some(i),
        Err(e) => {
            log_write(format!("Failed to read u8: '{}'",e), LogLevel::ERROR);
            None
        }
    }
}

pub fn read_u16(rdr: &mut Cursor<&Vec<u8>>) -> Option<u16> {
    match rdr.read_u16::<LittleEndian>() {
        Ok(i) => Some(i),
        Err(e) => {
            log_write(format!("Failed to read u16: '{}'",e), LogLevel::ERROR);
            None
        }
    }
}

pub fn read_i16(rdr: &mut Cursor<&Vec<u8>>) -> Option<i16> {
    match rdr.read_i16::<LittleEndian>() {
        Ok(i) => Some(i),
        Err(e) => {
            log_write(format!("Failed to read i16: '{}'",e), LogLevel::ERROR);
            None
        }
    }
}

pub fn read_u32(rdr: &mut Cursor<&Vec<u8>>) -> Option<u32> {
    match rdr.read_u32::<LittleEndian>() {
        Ok(i) => Some(i),
        Err(e) => {
            log_write(format!("Failed to read u32: '{}'",e), LogLevel::ERROR);
            None
        }
    }
}

pub fn is_debug() -> bool {
    let args: Vec<String> = env::args().collect();
    args.len() >= 2 && args[1] == "--debug"
}

#[cfg(test)]
mod tests_utils {
    use super::*;

    #[test]
    fn test_sanity() {
        assert_eq!(true,true);
    }

    #[test]
    fn test_abs() {
        let mut correct: PathBuf = PathBuf::from("yids_extract");
        correct.push("files");
        correct.push("file");
        correct.push("test.bin");
        let maybe = nitrofs_abs(&PathBuf::from("yids_extract"),&"test.bin".to_owned());
        assert_eq!(correct,maybe);
    }
}
