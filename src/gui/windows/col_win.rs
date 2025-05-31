use egui::{Color32, Image, Pos2, Rect, Response, Stroke, Vec2};

use crate::{data::{scendata::colz::draw_collision, types::CurrentLayer}, engine::displayengine::DisplayEngine, utils::{log_write, LogLevel}};

const TILES_WIDE: usize = 0x10;
const TILES_HIGH: usize = 0x10;
const COL_TILE_DIM: f32 = 16.0;
const COLL_RECT: Vec2 = Vec2::new(COL_TILE_DIM, COL_TILE_DIM);

pub fn collision_tiles_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    if de.display_settings.current_layer != CurrentLayer::Collision {
        ui.disable();
    }
    let top_left: Pos2 = ui.min_rect().min;
    let mut col_type_index: usize = 0;
    ui.allocate_space(Vec2 { x:260.0, y: 000.0 });
    for y in 0..TILES_HIGH {
        for x in 0..TILES_WIDE {
            let painter = ui.painter();
            let true_position: Pos2 = top_left + Vec2::new((x as f32) * COL_TILE_DIM, (y as f32) * COL_TILE_DIM);
            let rect: Rect = Rect::from_min_size(true_position, COLL_RECT);
            if col_type_index > u8::MAX as usize {
                log_write(format!("Col Type is too high: 0x{:X}",col_type_index), LogLevel::ERROR);
            } else {
                // Draw the tile
                let selected = de.col_tile_to_place as usize == col_type_index;
                if col_type_index == 0x1A { // COIN
                    let image: Image<'_> = Image::new(egui::include_image!("../../../assets/collision_coin.png")).tint(Color32::LIGHT_BLUE);
                    image.paint_at(ui, rect);
                } else {
                    draw_collision(painter, &rect, col_type_index as u8);
                }
                if selected {
                    painter.rect_stroke(rect, 0.0, Stroke::new(1.5, Color32::RED), egui::StrokeKind::Inside);
                }
            }
            col_type_index += 1;
        }
    }
    ui.add_space(260.0);
    // Interactivity
    if de.display_settings.current_layer == CurrentLayer::Collision {
        let click_response: Response = ui.interact(ui.min_rect(), egui::Id::new("col_window_tile_click"), egui::Sense::click());
        if click_response.clicked() {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - ui.min_rect().min;
                let tile_x: u32 = (local_pos.x/COL_TILE_DIM) as u32;
                let tile_y: u32 = (local_pos.y/COL_TILE_DIM) as u32;
                let tile_index: u32 = tile_y * (TILES_WIDE as u32) + tile_x;
                if tile_index > u8::MAX as u32 {
                    log_write(format!("Collision tile index out of bounds: 0x{:X}",tile_index), LogLevel::DEBUG);
                } else {
                    log_write(format!("Set collision tile placer to type 0x{:X}",tile_index), LogLevel::LOG);
                    de.col_tile_to_place = tile_index as u8;
                }
            }
        }
    }
}
