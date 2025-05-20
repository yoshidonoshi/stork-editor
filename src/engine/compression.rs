use std::{fs, path::PathBuf};

use crate::utils::{log_write, LogLevel};
use byteorder::{LittleEndian, WriteBytesExt};
use lamezip77::{self, nintendo_lz::Compress, VecBuf};

pub fn decompress_file(file_path: &PathBuf) -> Vec<u8> {
    let data_res = fs::read(file_path);
    if data_res.is_err() {
        // TODO: DEFINITELY do Err here
        log_write(format!("Could not decompress file at path: '{}', reason: '{}'",file_path.display(),data_res.unwrap_err()), LogLevel::FATAL);
        return Vec::new();
    }
    let data: Vec<u8> = data_res.unwrap();
    let first_byte = data[0];
    if first_byte != 0x10 {
        log_write("First byte was not 0x10".to_owned(), LogLevel::WARN);
    }
    let res: Vec<u8> = lamezip77_lz10_decomp(&data);
    res
}

pub fn lamezip77_lz10_decomp(data: &Vec<u8>) -> Vec<u8> {
    let mut vec_buf: VecBuf = VecBuf::new(0, u32::MAX as usize);
    {
        lamezip77::nintendo_lz::decompress_make!(decompressor,&mut vec_buf);
        let _ = decompressor.add_inp(data);
    }
    let ret: Vec<u8> = vec_buf.into();
    ret
}

/// Also includes the 0x10 magic number and uncompressed length
pub fn lamezip77_lz10_recomp(data: &Vec<u8>) -> Vec<u8> {
    let mut compressor = Compress::new();
    let mut output: Vec<u8> = Vec::new();
    let og_data_len = data.len();
    let first = og_data_len % 0x100;
    let second = (og_data_len >> 8) % 0x100;
    let third = (og_data_len >> 16) % 0x100;
    output.push(0x10);
    output.push(first as u8);
    output.push(second as u8);
    output.push(third as u8);
    compressor.compress(true, data, true, |val| {
        output.push(val);
    });
    output
}

// #[allow(dead_code)]
// pub fn cue_compress(data: &Vec<u8>) -> Vec<u8> {
//     //log_write(format!("Performing CUE compression"), LogLevel::LOG);
//     let temp_filename = String::from("TEMP_COMP.bin");
//     let write_attempt = fs::write(&temp_filename, data);
//     if write_attempt.is_err() {
//         log_write(format!("Temporary write failed: '{}'",write_attempt.unwrap_err()), LogLevel::disabled);
//     }
//     let mut executable_path = PathBuf::from("lib");
//     executable_path.push("cue-lzss");
//     let executable_path_abs = absolute(&executable_path);
//     if executable_path_abs.is_err() {
//         log_write(format!("Failed to create cue-lzss path: {}",executable_path_abs.unwrap_err()), LogLevel::disabled);
//     }
//     let executable_path = absolute(&executable_path).expect("Failed to unwrap executable path absolute");

//     let mut call = Command::new(executable_path);
//     call.arg("-evn");
//     call.arg(temp_filename.clone());
//     let call_result = call.output();
//     if call_result.as_ref().is_err() {
//         log_write(format!("Failed to execute call: '{}'",&call_result.as_ref().unwrap_err()), LogLevel::disabled);
//     }
//     let call_result = call_result.unwrap();
//     let call_result_string = String::from_utf8(call_result.stdout.clone());
//     if call_result_string.is_err() {
//         log_write(format!("Failed to get string from STDOUT: '{}'",call_result_string.as_ref().unwrap_err()), LogLevel::disable);
//     }
//     let call_result_string = call_result_string.as_ref().unwrap();
//     if !call_result_string.contains("Done") {
//         log_write("Compression call did not return 'Done', instead:".to_owned(), LogLevel::WARN);
//     }
//     let data = fs::read(temp_filename);
//     if data.is_err() {
//         log_write(format!("Failed to read compressed file: '{}'",data.as_ref().unwrap_err()), LogLevel::disable);
//     }
//     let remove = fs::remove_file("TEMP_COMP.bin");
//     if remove.is_err() {
//         log_write(format!("Failed to delete compression temp file: '{}'",remove.unwrap_err()),LogLevel::ERROR);
//     }
//     data.unwrap()
// }

pub fn segment_wrap(data: &Vec<u8>, magic: String) -> Vec<u8> {
    let mut ret: Vec<u8> = vec![];
    if magic.len() != 4 {
        log_write(format!("Header String was not length 4 in segment_wrap, was '{}'",magic), LogLevel::ERROR);
        return ret;
    }
    let mut internal_data = data.clone();
    while internal_data.len() % 4 != 0 {
        internal_data.push(0x00);
    }
    let internal_len = internal_data.len() as u32;
    let mut magic_vec: Vec<u8> = magic.into_bytes(); // Gets emptied so must be mut
    ret.append(&mut magic_vec);
    let _ = ret.write_u32::<LittleEndian>(internal_len);
    let mut to_insert = internal_data.clone();
    ret.append(&mut to_insert);
    while ret.len() % 4 != 0 {
        ret.push(0x00);
    }
    ret
}

pub fn segment_wrap_u32(data: &Vec<u8>, magic: u32) -> Vec<u8> {
    let mut ret: Vec<u8> = vec![];
    let mut internal_data = data.clone();
    while internal_data.len() % 4 != 0 {
        internal_data.push(0x00);
    }
    let internal_len = internal_data.len() as u32;
    let _ = ret.write_u32::<LittleEndian>(magic);
    let _ = ret.write_u32::<LittleEndian>(internal_len);
    let mut to_insert = internal_data.clone();
    ret.append(&mut to_insert);
    ret
}
