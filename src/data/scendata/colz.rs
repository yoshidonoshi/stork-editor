use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::{engine::compression::{lamezip77_lz10_decomp, lamezip77_lz10_recomp, segment_wrap}, utils::{log_write, LogLevel}};

use super::{info::ScenInfoData, ScenSegment};

pub const COLLISION_BG_COLOR: Color32 = Color32::from_rgba_premultiplied(0x40, 0x40, 0x60, 0x40);
pub const COLLISION_BG_COLOR_PASSABLE: Color32 = Color32::from_rgba_premultiplied(0x10, 0x40, 0x10, 0x40);
pub const COLLISION_BG_COLOR_LAVA: Color32 = Color32::from_rgba_premultiplied(0x80, 0x00, 0x00, 0x40);
pub const COLLISION_BG_COLOR_WATER_STILL: Color32 = Color32::from_rgba_premultiplied(0x00, 0x00, 0x80, 0x80);
pub const COLLISION_BG_COLOR_SOFT_ROCK: Color32 = Color32::from_rgba_premultiplied(0x80, 0x80, 0x00, 0x40);
pub const COLLISION_OUTLINE_COLOR: Color32 = Color32::from_rgba_premultiplied(0x40, 0x40, 0x60, 0xff);
pub const COLLISION_SQUARE: Vec2 = Vec2::new(16.0, 16.0);

#[derive(Debug,Clone,PartialEq,Default)]
pub struct CollisionData {
    /// Just keep it the same, it's just u8s
    pub col_tiles: Vec<u8>
}

impl CollisionData {
    pub fn new(compressed_buffer: &[u8]) -> Self {
        let decomp: Vec<u8> = lamezip77_lz10_decomp(compressed_buffer);
        Self {
            col_tiles: decomp
        }
    }
    pub fn increase_width(&mut self, old_width: u16, increase_by: usize) {
        // Tiles are 2x2
        if increase_by % 2 != 0 {
            log_write(format!("increase_by was not even: 0x{:X}",increase_by), LogLevel::Error);
            return;
        }
        if old_width % 2 != 0 {
            log_write(format!("old_width was not even: 0x{:X}",old_width), LogLevel::Error);
            return;
        }
        let increase_by = increase_by / 2;
        let old_width = old_width / 2;
        // Do increase logic
        let mut idx: usize = old_width as usize;
        // Do loop here
        while idx <= self.col_tiles.len() {
            for _ in 0..increase_by {
                self.col_tiles.insert(idx, 0x00);
            }
            idx = idx + (old_width as usize) + increase_by;
        }
    }
    pub fn decrease_width(&mut self, old_width: i32, decrease_by: i32) {
        // Tiles are 2x2
        if decrease_by % 2 != 0 {
            log_write(format!("decrease_by was not even: 0x{:X}",decrease_by), LogLevel::Error);
            return;
        }
        if old_width % 2 != 0 {
            log_write(format!("old_width was not even: 0x{:X}",old_width), LogLevel::Error);
            return;
        }
        let decrease_by = decrease_by / 2;
        let old_width = old_width / 2;
        let mut idx: i32 = old_width - 1;
        while idx < self.col_tiles.len() as i32 {
            for _ in 0..decrease_by {
                self.col_tiles.remove(idx as usize);
                idx -= 1;
            }
            idx += old_width;
        }
    }
    pub fn change_height(&mut self, new_height: u16, current_width: u16) {
        log_write(format!("Changing COLZ height to {:X}",new_height), LogLevel::Debug);
        let new_len = (new_height as u32 / 2) * (current_width as u32 / 2);
        self.col_tiles.resize(new_len as usize, 0x00);
    }
}

impl ScenSegment for CollisionData {
    fn compile(&self, _info: Option<&ScenInfoData>) -> Vec<u8> {
        self.col_tiles.clone()
    }

    fn wrap(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        let compiled = self.compile(info);
        let compressed = lamezip77_lz10_recomp(&compiled);
        segment_wrap(compressed, self.header())
    }

    fn header(&self) -> String {
        String::from("COLZ")
    }
}

fn draw_collision_polygon(painter: &Painter, pos_vec: Vec<Pos2>, bg_color: Color32) {
    let shap = Shape::convex_polygon(pos_vec, bg_color, Stroke::new(1.0, COLLISION_OUTLINE_COLOR));
    painter.add(shap);
}

pub fn draw_collision(painter: &Painter, rect: &Rect, col_type: u8) {
    puffin::profile_function!();

    let lt = rect.left_top();
    let rt = rect.right_top();
    let rb = rect.right_bottom();
    let lb = rect.left_bottom();
    let rc = rect.right_center();
    let lc = rect.left_center();
    let ct = rect.center_top();
    let cb = rect.center_bottom();

    match col_type {
        0x00 => { /* Blank */ },
        0x01 => draw_collision_polygon(painter, vec![lt,rt,rb,lb],COLLISION_BG_COLOR),
        0x02 => draw_collision_polygon(painter, vec![lt,rt,rb,lb],COLLISION_BG_COLOR_PASSABLE),
        0x03 => draw_collision_polygon(painter, vec![lb,rc,rb],COLLISION_BG_COLOR),
        0x04 => draw_collision_polygon(painter, vec![lc,rt,rb,lb],COLLISION_BG_COLOR),
        0x05 => draw_collision_polygon(painter, vec![lb,ct,rt,rb],COLLISION_BG_COLOR),
        0x06 => draw_collision_polygon(painter, vec![rt,rb,cb],COLLISION_BG_COLOR),
        0x07 => draw_collision_polygon(painter, vec![lb,rt,rb],COLLISION_BG_COLOR),
        0x09 => draw_collision_polygon(painter, vec![lt,rt,rb,lb], COLLISION_BG_COLOR_LAVA),
        0x12 => draw_collision_polygon(painter, vec![lt,rt,rb,lb], COLLISION_BG_COLOR_WATER_STILL),
        0x14 => draw_collision_polygon(painter, vec![lb,rc,rb], COLLISION_BG_COLOR_PASSABLE),
        0x15 => draw_collision_polygon(painter, vec![lc,rt,rb,lb], COLLISION_BG_COLOR_PASSABLE),
        0x16 => draw_collision_polygon(painter, vec![lb,ct,rt,rb], COLLISION_BG_COLOR_PASSABLE),
        0x17 => draw_collision_polygon(painter, vec![cb,rt,rb], COLLISION_BG_COLOR_PASSABLE),
        0x18 => draw_collision_polygon(painter, vec![lb,rt,rb], COLLISION_BG_COLOR_PASSABLE),
        0x1A => { /* Coin */ },
        0x1B => draw_collision_polygon(painter, vec![lt,rt,rb,lb], COLLISION_BG_COLOR_SOFT_ROCK),
        0x1F => draw_collision_polygon(painter, vec![lb,rt,rb], COLLISION_BG_COLOR_PASSABLE),
        0x43 => draw_collision_polygon(painter, vec![lc,rb,lb],COLLISION_BG_COLOR),
        0x44 => draw_collision_polygon(painter, vec![lt,rc,rb,lb],COLLISION_BG_COLOR),
        0x45 => draw_collision_polygon(painter, vec![lt,ct,rb,lb],COLLISION_BG_COLOR),
        0x46 => draw_collision_polygon(painter, vec![lt,cb,lb],COLLISION_BG_COLOR),
        0x47 => draw_collision_polygon(painter, vec![lt,rb,lb],COLLISION_BG_COLOR),
        0x54 => draw_collision_polygon(painter, vec![lc,rb,lb], COLLISION_BG_COLOR_PASSABLE),
        0x55 => draw_collision_polygon(painter, vec![lt,rc,rb,lb], COLLISION_BG_COLOR_PASSABLE),
        0x56 => draw_collision_polygon(painter, vec![lt,ct,rb,lb], COLLISION_BG_COLOR_PASSABLE),
        0x57 => draw_collision_polygon(painter, vec![lt,cb,lb], COLLISION_BG_COLOR_PASSABLE),
        0x58 => draw_collision_polygon(painter, vec![lt,rb,lb], COLLISION_BG_COLOR_PASSABLE),
        0x83 => draw_collision_polygon(painter, vec![lt,rt,rc], COLLISION_BG_COLOR),
        0x84 => draw_collision_polygon(painter, vec![lt,rt,rb,lc], COLLISION_BG_COLOR),
        0x85 => draw_collision_polygon(painter, vec![lt,rt,rb,cb], COLLISION_BG_COLOR),
        0x86 => draw_collision_polygon(painter, vec![ct,rt,rb], COLLISION_BG_COLOR),
        0x87 => draw_collision_polygon(painter, vec![lt,rt,rb],COLLISION_BG_COLOR),
        0xC3 => draw_collision_polygon(painter, vec![lt,rt,lc],COLLISION_BG_COLOR),
        0xC4 => draw_collision_polygon(painter, vec![lt,rt,rc,lb],COLLISION_BG_COLOR),
        0xC5 => draw_collision_polygon(painter, vec![lt,rt,cb,lb], COLLISION_BG_COLOR),
        0xC6 => draw_collision_polygon(painter, vec![lt,ct,lb], COLLISION_BG_COLOR),
        0xC7 => draw_collision_polygon(painter, vec![lt,rt,lb],COLLISION_BG_COLOR),
        _ => {
            // Unknown, put text
            painter.rect_filled(*rect, 0.0, COLLISION_BG_COLOR);
            painter.text(
                lt + Vec2::new(1.0, 1.0), Align2::LEFT_TOP,
                format!("{:02X}",col_type),
                FontId { size: 12.0, family: egui::FontFamily::Monospace },
                Color32::BLACK
            );
            painter.text(
                lt, Align2::LEFT_TOP,
                format!("{:02X}",col_type),
                FontId { size: 12.0, family: egui::FontFamily::Monospace },
                Color32::WHITE
            );
        }
    }
}
