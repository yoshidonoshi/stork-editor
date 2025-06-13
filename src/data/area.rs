use std::{fmt, io::Cursor};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use uuid::Uuid;

use egui::{Color32, Pos2, Rect, Vec2};

use crate::{engine::compression::segment_wrap, utils::{log_write, LogLevel}};

use super::{Compilable, TopLevelSegment};

pub const AREA_RECT_COLOR: Color32 = Color32::from_rgba_premultiplied(0x60, 0x00, 0x00, 0x40);
pub const AREA_RECT_COLOR_SELECTED: Color32 = Color32::from_rgba_premultiplied(0x80, 0x10, 0x10, 0x50);

#[derive(Debug,Clone,PartialEq,Default)]
pub struct TriggerData {
    pub triggers: Vec<Trigger>
}
impl fmt::Display for TriggerData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Maybe print them all out at some point?
        write!(f,"Trigger [ TriggerCount=0x{:X}/{} ]",self.triggers.len(),self.triggers.len())
    }
}
impl TopLevelSegment for TriggerData {
    fn compile(&self) -> Vec<u8> {
        self.triggers.iter().flat_map(|trigger| trigger.compile()).collect()
    }
    // No compression
    fn wrap(&self) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile();
        segment_wrap(comp_bytes, "AREA".to_owned())
    }

    fn header(&self) -> String {
        String::from("AREA")
    }
}
impl TriggerData {
    pub fn new(byte_data: &[u8]) -> Self {
        let mut rdr = Cursor::new(byte_data);
        let seg_end: usize = byte_data.len();
        let mut ret: TriggerData = TriggerData::default(); // Empty
        while rdr.position() < seg_end as u64 {
            let left_x = match rdr.read_u16::<LittleEndian>() {
                Err(error) => {
                    log_write(format!("Error reading LeftX for TriggerData: '{}'", error), LogLevel::Error);
                    break;
                }
                Ok(left_x) => left_x,
            };
            let top_y = rdr.read_u16::<LittleEndian>().expect("top_y in TriggerData");
            let right_x = rdr.read_u16::<LittleEndian>().expect("right_x in TriggerData");
            let bottom_y = rdr.read_u16::<LittleEndian>().expect("bottom_y in TriggerData");
            let t = Trigger::new(left_x, top_y, right_x, bottom_y);
            ret.triggers.push(t);
        }
        ret
    }

    pub fn delete(&mut self, uuid: Uuid) -> bool {
        if let Some(pos) = self.triggers.iter().position(|x| x.uuid == uuid) {
            self.triggers.remove(pos);
            log_write("Trigger data deleted", LogLevel::Debug);
            true
        } else {
            false
        }

    }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Trigger {
    pub left_x: u16,
    pub top_y: u16,
    pub right_x: u16,
    pub bottom_y: u16,
    pub uuid: Uuid
}
impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"Trigger [ LeftX=0x{}, TopY=0x{}, RightX=0x{}, BottomY=0x{} ]",
            self.left_x,self.top_y,self.right_x,self.bottom_y)
    }
}
impl Compilable for Trigger {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u16::<LittleEndian>(self.left_x);
        let _ = comp.write_u16::<LittleEndian>(self.top_y);
        let _ = comp.write_u16::<LittleEndian>(self.right_x);
        let _ = comp.write_u16::<LittleEndian>(self.bottom_y);
        comp
    }
}
impl Trigger {
    pub fn new(left_x: u16,top_y: u16,right_x: u16,bottom_y: u16) -> Self {
        Self {
            left_x, top_y, right_x, bottom_y, uuid: Uuid::new_v4()
        }
    }
    pub fn get_rect(&self, top_left_screen: Pos2, tile_width_px: f32, tile_height_px: f32) -> Rect {
        let top_left = Vec2::new(
            self.left_x as f32 * tile_width_px,
            self.top_y as f32 * tile_height_px
        );
        let bottom_right = Vec2::new(
            self.right_x as f32 * tile_width_px,
            self.bottom_y as f32 * tile_height_px
        );
        Rect::from_two_pos(top_left_screen + top_left, top_left_screen + bottom_right)
    }
}

pub struct TriggerSettings {
    pub selected_uuid: Uuid
}
impl Default for TriggerSettings {
    fn default() -> Self {
        Self {
            selected_uuid: Uuid::nil()
        }
    }
}
