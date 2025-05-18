use egui::{Align2, Color32, FontId, Pos2, Rect, Vec2};

use crate::engine::displayengine::DisplayEngine;

const PAL_BOX_WIDTH: f32 = 15.0;
const PAL_BOX_HEIGHT: f32 = 15.0;
const PAL_RECT: Vec2 = Vec2::new(PAL_BOX_WIDTH, PAL_BOX_HEIGHT);

pub fn palette_window_show(ui: &mut egui::Ui, de: &DisplayEngine) {
    let top_left: Pos2 = ui.min_rect().min;
    for y in 0..16 {
        for x in 0..16 {
            let col = &de.bg_palettes[y].colors[x].color;
            draw_rect(ui, (x as f32) * PAL_BOX_WIDTH, (y as f32) * PAL_BOX_HEIGHT, &PAL_RECT, *col);
        }
        ui.painter().text(
            Pos2::new(
                top_left.x + 242.0,
                top_left.y + 2.0 + (y as f32) * PAL_BOX_HEIGHT
            ),
            Align2::LEFT_TOP,
            format!("0x{:X}",y as u32),
            FontId::monospace(10.0),
            Color32::WHITE
        );
    }
    ui.add_space(242.0);
    let mut hover_label: String = String::from("N/A");
    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
        let mouse_pos: Vec2 = hover_pos - top_left;
        let mouse_x: u32 = (mouse_pos.x / PAL_BOX_WIDTH) as u32;
        let mouse_y: u32 = (mouse_pos.y / PAL_BOX_HEIGHT) as u32;
        let mut short_val: u16 = 0x0000;
        let mut addr_val: u32 = 0x00000000;
        if mouse_y < 16 && mouse_x < 16 {
            let cur_pal = &de.bg_palettes[mouse_y as usize];
            let cur_col = &cur_pal.colors[mouse_x as usize];
            short_val = cur_col._short;
            addr_val = cur_col._addr;
        }

        //println!("x: {:X}, y: {:X}",mouse_x,mouse_y);
        if mouse_x <= 0xF && mouse_y <= 0xF {
            hover_label = format!("BGP {:X} - Color {:X} - 0x{:04X} - 0x{:08X}",mouse_y,mouse_x,short_val,addr_val);
        }
    }
    ui.label(hover_label);
}

fn draw_rect(ui: &mut egui::Ui, pos_x: f32, pos_y: f32, dimensions: &Vec2, color: Color32) {
    let painter: &egui::Painter = ui.painter();
    let top_left: Pos2 = ui.min_rect().min;
    let true_position: Pos2 = top_left + Vec2::new(pos_x, pos_y);
    let rect: Rect = Rect::from_min_size(true_position, *dimensions);
    painter.rect_filled(rect, 0.0, color);
}