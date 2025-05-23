use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::engine::compression::{lamezip77_lz10_decomp, lamezip77_lz10_recomp, segment_wrap};

use super::{info::ScenInfoData, ScenSegment};

pub const COLLISION_BG_COLOR: Color32 = Color32::from_rgba_premultiplied(0x40, 0x40, 0x60, 0x40);
pub const COLLISION_BG_COLOR_PASSABLE: Color32 = Color32::from_rgba_premultiplied(0x10, 0x40, 0x10, 0x40);
pub const COLLISION_OUTLINE_COLOR: Color32 = Color32::from_rgba_premultiplied(0x40, 0x40, 0x60, 0xff);
pub const COLLISION_SQUARE: Vec2 = Vec2::new(16.0, 16.0);

#[derive(Debug,Clone,PartialEq,Default)]
pub struct CollisionData {
    /// Just keep it the same, it's just u8s
    pub col_tiles: Vec<u8>
}

impl CollisionData {
    pub fn new(compressed_buffer: &Vec<u8>) -> Self {
        let mut ret: CollisionData = CollisionData::default();
        let decomp: Vec<u8> = lamezip77_lz10_decomp(compressed_buffer);
        ret.col_tiles = decomp;

        ret
    }
}

impl ScenSegment for CollisionData {
    fn compile(&self, _info: &Option<ScenInfoData>) -> Vec<u8> {
        self.col_tiles.clone()
    }

    fn wrap(&self, info: &Option<ScenInfoData>) -> Vec<u8> {
        let compiled = self.compile(info);
        let compressed = lamezip77_lz10_recomp(&compiled);
        segment_wrap(&compressed, self.header())
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
    match col_type {
        0x00 => { /* Blank */ },
        0x01 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_top(),rect.right_bottom(),rect.left_bottom()],COLLISION_BG_COLOR),
        0x02 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_top(),rect.right_bottom(),rect.left_bottom()],COLLISION_BG_COLOR_PASSABLE),
        0x03 => draw_collision_polygon(painter, vec![rect.left_bottom(),rect.right_center(),rect.right_bottom()],COLLISION_BG_COLOR),
        0x04 => draw_collision_polygon(painter, vec![rect.left_center(),rect.right_top(),rect.right_bottom(),rect.left_bottom()],COLLISION_BG_COLOR),
        0x05 => draw_collision_polygon(painter, vec![rect.left_bottom(),rect.center_top(),rect.right_top(),rect.right_bottom()],COLLISION_BG_COLOR),
        0x06 => draw_collision_polygon(painter, vec![rect.right_top(),rect.right_bottom(),rect.center_bottom()],COLLISION_BG_COLOR),
        0x07 => draw_collision_polygon(painter, vec![rect.left_bottom(),rect.right_top(),rect.right_bottom()],COLLISION_BG_COLOR),
        0x1A => { /* Coin */ },
        0x43 => draw_collision_polygon(painter, vec![rect.left_center(),rect.right_bottom(),rect.left_bottom()],COLLISION_BG_COLOR),
        0x44 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_center(),rect.right_bottom(),rect.left_bottom()],COLLISION_BG_COLOR),
        0x45 => draw_collision_polygon(painter, vec![rect.left_top(),rect.center_top(),rect.right_bottom(),rect.left_bottom()],COLLISION_BG_COLOR),
        0x46 => draw_collision_polygon(painter, vec![rect.left_top(),rect.center_bottom(),rect.left_bottom()],COLLISION_BG_COLOR),
        0x47 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_bottom(),rect.left_bottom()],COLLISION_BG_COLOR),
        0x87 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_top(),rect.right_bottom()],COLLISION_BG_COLOR),
        0xC3 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_top(),rect.left_center()],COLLISION_BG_COLOR),
        0xC4 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_top(),rect.right_center(),rect.left_bottom()],COLLISION_BG_COLOR),
        0xC7 => draw_collision_polygon(painter, vec![rect.left_top(),rect.right_top(),rect.left_bottom()],COLLISION_BG_COLOR),
        _ => {
            // Unknown, put text
            painter.rect_filled(*rect, 0.0, COLLISION_BG_COLOR);
            painter.text(
                rect.left_top()+Vec2::new(1.0, 1.0), Align2::LEFT_TOP,
                format!("{:02X}",col_type),
                FontId { size: 12.0, family: egui::FontFamily::Monospace },
                Color32::BLACK
            );
            painter.text(
                rect.left_top(), Align2::LEFT_TOP,
                format!("{:02X}",col_type),
                FontId { size: 12.0, family: egui::FontFamily::Monospace },
                Color32::WHITE
            );
        }
    }
}
