use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use uuid::Uuid;

use crate::{engine::compression::segment_wrap, utils::{log_write, LogLevel}};

use super::{Compilable, TopLevelSegment};

#[derive(Debug,Clone,PartialEq,Default)]
pub struct PathDatabase {
    pub path_count: u32,
    pub lines: Vec<PathLine>
}
impl PathDatabase {
    pub fn new(byte_data: &Vec<u8>) -> Self {
        let mut ret = PathDatabase::default();
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(byte_data);
        let path_count = rdr.read_u32::<LittleEndian>();
        if path_count.is_err() {
            log_write(format!("Failed to get path_count from PathDatabase: '{}'",path_count.unwrap_err()), LogLevel::ERROR);
            return ret;
        }
        ret.path_count = path_count.unwrap();
        let mut path_index: u32 = 0;
        while path_index < ret.path_count { // Build the line
            // Don't use default, it adds a blank point
            let mut points: Vec<PathPoint> = Vec::new();
            loop { // Build the points
                let angle = rdr.read_i16::<LittleEndian>();
                if angle.is_err() {
                    log_write(format!("Failed to read Path angle: '{}'",angle.unwrap_err()), LogLevel::ERROR);
                    return ret;
                }
                let angle = angle.unwrap();
                let distance = rdr.read_i16::<LittleEndian>().expect("distance i16 in PathDatabase");
                let x_fine = rdr.read_u32::<LittleEndian>().expect("x_fine u32 in PathDatabase");
                let y_fine = rdr.read_u32::<LittleEndian>().expect("y_fine u32 in PathDatabase");
                let point = PathPoint::new(angle, distance, x_fine, y_fine);
                points.push(point);
                if distance == 0x0000 {
                    break;
                }
            }
            ret.lines.push(PathLine { points: points, uuid: Uuid::new_v4() });
            path_index += 1;
        }
        ret
    }

    pub fn delete_line(&mut self, line_uuid: Uuid) -> Result<(),()> {
        log_write("Deleting Line", LogLevel::DEBUG);
        let line_pos = self.lines.iter().position(|x| x.uuid == line_uuid);
        if line_pos.is_none() {
            return Err(());
        }
        let line_pos = line_pos.unwrap();
        self.lines.remove(line_pos);
        log_write("Line data deleted", LogLevel::DEBUG);
        Ok(())
    }
    pub fn fix_term(&mut self) {
        for line in &mut self.lines {
            if line.points.is_empty() {
                // No empty Lines
                line.points.push(PathPoint::default());
            } else {
                // Don't let any be zero... except the last
                for p in &mut line.points {
                    if p.distance == 0 {
                        p.distance = 1;
                    }
                }
                // End must have distance 0
                let line_len = line.points.len();
                line.points[line_len-1].distance = 0;
            }
        }
    }
}
impl TopLevelSegment for PathDatabase {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u32::<LittleEndian>(self.lines.len() as u32);
        for line in &self.lines {
            for point in &line.points {
                let mut p = point.compile();
                comp.append(&mut p);
            }
        }
        comp
    }

    fn wrap(&self) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile();
        segment_wrap(&comp_bytes, "PATH".to_owned())
    }

    fn header(&self) -> String {
        String::from("PATH")
    }
}

#[derive(Debug,Clone,PartialEq)]
pub struct PathLine {
    pub points: Vec<PathPoint>,
    pub uuid: Uuid
}
impl Default for PathLine {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            uuid: Uuid::new_v4()
        }
    }
}

pub struct PathSettings {
    pub selected_line: Uuid,
    pub selected_point: Uuid
}
impl Default for PathSettings {
    fn default() -> Self {
        Self {
            selected_line: Uuid::nil(),
            selected_point: Uuid::nil()
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct PathPoint {
    pub angle: i16,
    pub distance: i16,
    pub x_fine: u32,
    pub y_fine: u32,
    pub uuid: Uuid
}
impl PathPoint {
    pub fn new(angle: i16, distance: i16, x_fine: u32, y_fine: u32) -> Self {
        Self {
            angle, distance, x_fine, y_fine, uuid: Uuid::new_v4()
        }
    }
}
impl Default for PathPoint {
    fn default() -> Self {
        Self {
            angle: 0, distance: 0x0000,
            x_fine: 0, y_fine: 0,
            uuid: Uuid::new_v4()
        }
    }
}
impl Compilable for PathPoint {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_i16::<LittleEndian>(self.angle);
        let _ = comp.write_i16::<LittleEndian>(self.distance);
        let _ = comp.write_u32::<LittleEndian>(self.x_fine);
        let _ = comp.write_u32::<LittleEndian>(self.y_fine);
        comp
    }
}
