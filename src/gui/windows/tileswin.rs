use egui::{pos2, Color32, Pos2, Rect, TextureHandle, Vec2};

use crate::utils::{log_write, LogLevel};


const TILE_BOX_WIDTH: f32 = 2.0;
const TILE_WIDTH: f32 = TILE_BOX_WIDTH*8.0;
const TILE_BOX_HEIGHT: f32 = 2.0;
const TILE_HEIGHT: f32 = TILE_BOX_HEIGHT*8.0;
const TILE_RECT: Vec2 = Vec2::new(TILE_WIDTH, TILE_HEIGHT);
const TILES_ARRAY_WIDTH: usize = 0x10;
const TOP_MARGIN: f32 = 1.0;

pub fn tiles_window_show(ui: &mut egui::Ui, preview_tile_cache: &Vec<TextureHandle>) {
    puffin::profile_function!();
    let painter: &egui::Painter = ui.painter();
    let top_left: Pos2 = ui.min_rect().min + Vec2::new(0.0, TOP_MARGIN);
    for (tile_index,tile) in preview_tile_cache.iter().enumerate() {
        let tex_id = &tile.id();
        let tile_col_offset = (tile_index % TILES_ARRAY_WIDTH) as f32 * TILE_WIDTH;
        let tile_row_offset = (tile_index / TILES_ARRAY_WIDTH) as f32 * TILE_HEIGHT;
        // Do the render
        let rect: Rect = Rect::from_min_size(top_left + Vec2::new(tile_col_offset, tile_row_offset), TILE_RECT);
        painter.image(*tex_id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    // Add more clickable space
    ui.allocate_space(Vec2::new(300.0, 0.0));
    let click_response = ui.interact(ui.min_rect(), egui::Id::new("Tiles_Window_Click"), egui::Sense::click());
    if click_response.clicked() {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
            let local_pos = pointer_pos - ui.min_rect().min;
            let base_tile_x: u32 = (local_pos.x/TILE_WIDTH) as u32;
            let base_tile_y: u32 = (local_pos.y/TILE_HEIGHT) as u32;
            let tile_index = base_tile_x + (base_tile_y * TILES_ARRAY_WIDTH as u32);
            // Do something more with this eventually
            println!("pos: {}/{}: 0x{:X}",base_tile_x,base_tile_y,tile_index);
        } else {
            log_write("Unable to get pointer_pos in tileswin", LogLevel::ERROR);
        }
    }
}
