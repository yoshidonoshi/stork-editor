use byteorder::{LittleEndian, ReadBytesExt};

use std::io::{Cursor, Read};
use crate::utils::{self, header_to_string, log_write, string_to_header, LogLevel};

use std::fmt;

/// A struct representing stored data for the game. 
/// It follows the 4 byte header, 4 byte size, then internal bytes format
#[derive(Clone)]
pub struct DataSegment {
    pub header: u32,
    pub internal_data: Vec<u8>
}
impl Default for DataSegment {
    fn default() -> Self {
        Self { header: 0xdeadbeef, internal_data: vec![] }
    }
}
impl fmt::Display for DataSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header_str = header_to_string(&self.header);
        let internal_len = &self.internal_data.len();
        write!(f, "DataSegment [ header={}, internal_data.len=0x{:X}/{} ]", header_str, internal_len, internal_len )
    }
}
impl DataSegment {
    pub fn _new_from_bytes(input: &Vec<u8>) -> Self {
        let mut rdr = Cursor::new(input);
        let header = rdr.read_u32::<LittleEndian>().unwrap();
        let size: usize = rdr.read_u32::<LittleEndian>().unwrap() as usize;
        let mut inner_data: Vec<u8> = Vec::new();
        if let Err(_) = rdr.read_to_end(&mut inner_data) {
            utils::log_write(String::from("Could not read to end of data"), utils::LogLevel::ERROR);
        }
        let inside_len = inner_data.len();
        if inside_len != size {
            println!("Mismatch in file specified internal size vs actual: 0x{:05X} vs 0x{:05X}", size, inside_len);
        }
        Self {
            header: header,
            internal_data: inner_data
        }
    }
    #[allow(dead_code)]
    pub fn new(input: &Vec<u8>, header: String) -> Self {
        if header.len() != 4 {
            log_write(format!("Bad header string length: '{}'",header.len()), LogLevel::ERROR);
        }
        Self {
            header: string_to_header(header),
            internal_data: input.clone()
        }
    }
}
