use std::{io::Cursor, path::PathBuf};
use byteorder::{LittleEndian, ReadBytesExt};
use utils::LogLevel;
use std::fmt;

use crate::{data::segments::DataSegment, engine::compression, utils::{self, log_write, nitrofs_abs}};

pub struct RenderArchive {
    pub segments: Vec<DataSegment>,
    pub src_file: String
}
impl Default for RenderArchive {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            src_file: String::from("ERROR")
        }
    }
}
impl fmt::Display for RenderArchive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seg_count = self.segments.len();
        write!(f,"RenderArchive [ src_file='{}', segments.len=0x{:X}/{} ]",self.src_file,seg_count,seg_count)
    }
}
impl RenderArchive {
    pub fn new(filename_local: String, project_directory: &PathBuf) -> Self {
        log_write(format!("Loading new RenderArchive '{}'",&filename_local), LogLevel::DEBUG);
        let true_path: PathBuf = nitrofs_abs(project_directory,&filename_local);
        let file_bytes: Vec<u8> = compression::decompress_file(&true_path);
        let mut rdr = Cursor::new(&file_bytes);
        let Ok(file_header) = rdr.read_u32::<LittleEndian>() else {
            utils::log_write("Error getting master header from RenderArchive".to_owned(), LogLevel::ERROR);
            return Self::default();
        };
        let header_string = utils::header_to_string(&file_header);
        if header_string != "OBAR" {
            utils::log_write(format!("RenderArchive master header was not OBAR, was instead '{}'",header_string), LogLevel::ERROR);
            return Self::default();
        }
        let _ = rdr.read_u32::<LittleEndian>().unwrap();
        let mut segments: Vec<DataSegment> = vec![];
        let file_end_pos: u64 = file_bytes.len() as u64;
        while rdr.position() < file_end_pos {
            let section_head: u32 = rdr.read_u32::<LittleEndian>().unwrap();
            let section_size: usize = rdr.read_u32::<LittleEndian>().unwrap() as usize;
            let mut internal_vec: Vec<u8> = vec![0;section_size];
            for i in 0..section_size {
                internal_vec[i] = rdr.read_u8().unwrap();
            }
            let cur_segment: DataSegment = DataSegment { header: section_head, internal_data: internal_vec };
            segments.push(cur_segment);
        }
        Self {
            segments,
            src_file: filename_local
        }
    }
}
