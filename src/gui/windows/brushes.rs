use std::fmt;

use egui::{Color32, Painter, Pos2, Rect, Response, RichText, Stroke, Vec2};
use serde::{Deserialize, Serialize};

use crate::{data::types::{MapTileRecordData, Palette}, engine::displayengine::DisplayEngine, utils::{color_image_from_pal, get_pixel_bytes_16, get_uvs_from_tile, log_write, pixel_byte_array_to_nibbles, LogLevel}};

#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct Brush {
    pub tileset: String,
    pub name: String,
    pub width: u8,
    pub height: u8,
    /// Is this needed?
    pub palette_offset: u8,
    pub tiles: Vec<u16>
}
impl Default for Brush {
    fn default() -> Self {
        Self {
            tileset: "char01c".to_owned(),
            name: "Blank/Example".to_owned(),
            width: 0,
            height: 0,
            palette_offset: 0,
            tiles: vec![]
        }
    }
}
impl fmt::Display for Brush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"Brush [ name='{}', tileset={}, width/height=0x{:X}/0x{:X}, first_tile={:04X} ]",
            self.name,self.tileset,self.width,self.height,self.tiles[0])
    }
}
impl Brush {
    pub fn clear(&mut self) {
        self.tiles.clear();
        self.height = 0;
        self.width = 0;
        self.name = String::from("NAME CLEARED");
    }
}

pub struct BrushSettings {
    pub cur_selected_brush: Option<usize>,
    pub pos_brush_name: String,
    pub cur_search_string: String,
    pub only_show_same_tileset: bool
}
impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            cur_selected_brush: Option::None,
            pos_brush_name: String::from("Untitled Brush"),
            cur_search_string: String::from(""),
            only_show_same_tileset: false
        }
    }
}

const BRUSH_TILE_DIM: f32 = 16.0;
const BRUSH_TILES_WIDE: i32 = 16;
const BRUSH_TILE_RECT: Vec2 = Vec2::new(BRUSH_TILE_DIM, BRUSH_TILE_DIM);

pub fn show_brushes_window(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    if !de.display_settings.is_cur_layer_bg() {
        // Technically uneccesary, but good for appearance
        ui.disable();
    }
    let top_left = ui.min_rect().min;
    ui.allocate_space(Vec2 { x:260.0, y: 000.0 });
    let cur_layer = de.display_settings.current_layer as u8;
    if cur_layer < 1 || cur_layer > 3 {
        ui.label("Not on a BG layer");
        return;
    }
    if let Some(layer) = de.loaded_map.get_background(cur_layer) {
        let pal = &layer.get_pltb();
        if pal.is_none() {
            return;
        }
        let info = layer.get_info().expect("brush layer has info");
        if let Some(tiles) = &layer.pixel_tiles_preview {
            do_tile_draw(
                ui, &mut de.current_brush, &de.bg_palettes,
                tiles,&info.color_mode,&layer._pal_offset
            );
        }
        let mut push_height: f32 = 260.0;
        let calced_height = de.current_brush.height as f32 * 16.0;
        if calced_height > push_height {
            push_height = calced_height;
        }
        ui.add_space(push_height);
        // Interactivity
        let click_response: Response = ui.interact(ui.min_rect(), egui::Id::new("saved_brushes_window_click"), egui::Sense::click());
        if click_response.secondary_clicked() {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - top_left;
                let tile_x: u32 = (local_pos.x/BRUSH_TILE_DIM) as u32;
                let mut should_delete: bool = true;
                if tile_x >= de.current_brush.width as u32 {
                    log_write("tile_x out of bounds for Brush", LogLevel::WARN);
                    should_delete = false;
                }
                let tile_y: u32 = (local_pos.y/BRUSH_TILE_DIM) as u32;
                if tile_y >= de.current_brush.height as u32 {
                    log_write("tile_y out of bounds for Brush", LogLevel::WARN);
                    should_delete = false;
                }
                let tile_index: u32 = tile_y * (de.current_brush.width as u32) + tile_x;
                if tile_index as usize >= de.current_brush.tiles.len() {
                    log_write(format!("Tile index too high for Brush: 0x{:X}",tile_index), LogLevel::WARN);
                    should_delete = false;
                }
                if should_delete {
                    de.current_brush.tiles[tile_index as usize] = 0x0000;
                }
            } else {
                log_write("Failed to get pointer input when clicking Saved Brushes grid", LogLevel::ERROR);
            }
        } // End interactivity
        // Helper for selection
        if !de.bg_sel_data.selected_map_indexes.is_empty() {
            let sel_width: u16 = de.bg_sel_data.selection_width;
            let sel_height: u16 = de.bg_sel_data.selection_height;
            let odd_size = sel_width % 2 != 0 || sel_height % 2 != 0;
            let raw_text = format!("Selection width/height: {}/{}",sel_width,sel_height);
            let top_left = de.bg_sel_data.get_top_left(layer.get_info().expect("Layer has INFO").layer_width);
            if top_left.is_none() {
                log_write("Unable to get top left from bg selection in brushes", LogLevel::ERROR);
                return;
            }
            let top_left = top_left.unwrap();
            let odd_pos = (top_left.x as u32) % 2 != 0 || (top_left.y as u32) % 2 != 0;
            let mut rich_text = RichText::new(raw_text);
            if odd_pos {
                rich_text = rich_text.color(Color32::RED).underline();
                let odd_pos_label = ui.label(rich_text);
                if odd_pos_label.hovered() {
                    egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("odd_sel_pos"), |ui| {
                        ui.label("The top left corner of your selection is odd, this is very unoptimal for Brushes");
                        ui.label("Tip: Use the red square near your cursor to locate the nearest top left for selecting");
                        ui.label("If the top left looks even, you may have selected invisible tiles");
                        ui.label("Tip: Use Control + Drag to remove excess selected tiles, and Shift + Drag to add more");
                    });
                }
            } else if odd_size {
                rich_text = rich_text.color(Color32::ORANGE).underline();
                let odd_size_label = ui.label(rich_text);
                if odd_size_label.hovered() {
                    egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("odd_sel_size"), |ui| {
                        ui.label("Your selection's dimensions are odd (not divisible by 2), this is unoptimal for Brushes");
                        ui.label("You can ignore this if the bottom left or right have a clear line");
                        ui.label("Tip: Use Control + Drag to remove excess selected tiles, and Shift + Drag to add more");
                    });
                }
            } else {
                let _good_label = ui.label(rich_text);
            }
        } else {
            ui.label("Selection width/height: N/A");
        }
        // Button panel
        ui.horizontal(|ui| {
            let mut label_str = String::from("Tile selection loadable");
            let mut load_tiles_enabled = true;
            if de.bg_sel_data.selection_width == 0 {
                label_str = String::from("Nothing selected");
                load_tiles_enabled = false;
            // } else if odd_pos { // Potential edge cases, leave disabled for now
            //     label_str = String::from("Selection top left is odd");
            //     load_tiles_enabled = false;
            } else if de.bg_sel_data.selection_width > 16 {
                label_str = String::from("Selection too wide (16 tiles max)");
                load_tiles_enabled = false;
            } else if (de.bg_sel_data.selected_map_indexes.len() / de.bg_sel_data.selection_width as usize) > 16 {
                // This can't divide by zero as it already checked if selection_width was 0
                label_str = String::from("Selection too tall (16 tiles max)");
                load_tiles_enabled = false;
            } else if de.bg_sel_data.selected_map_indexes.is_empty() {
                label_str = String::from("No tiles selected");
                load_tiles_enabled = false;
            }
            let load_tiles = ui.add_enabled(load_tiles_enabled,
                egui::Button::new("Load Selection"));
            ui.label(label_str);
            if load_tiles.clicked() {
                if de.bg_sel_data.selection_width == 0 {
                    log_write("Cannot load selected tiles, selection width is 0", LogLevel::WARN);
                    return;
                }
                if de.bg_sel_data.selected_map_indexes.is_empty() {
                    log_write("Cannot load selected tiles, nothing selected", LogLevel::WARN);
                    return;
                }
                let maptiles = layer.get_mpbz().clone().expect("maptiles should be Some'd on a layer");
                de.current_brush.tiles.clear();
                if de.bg_sel_data.selection_width >= u8::MAX as u16 {
                    log_write("Selection width higher than u8 able", LogLevel::ERROR);
                    return;
                }
                de.current_brush.width = de.bg_sel_data.selection_width as u8;
                let height = de.bg_sel_data.selected_map_indexes.len() as f32 / de.current_brush.width as f32;
                de.current_brush.height = height as u8;
                de.current_brush.tileset = info.imbz_filename_noext.clone().unwrap_or("N/A".to_string());
                for selected_index in &de.bg_sel_data.selected_map_indexes {
                    let tile_data = &maptiles.tiles[*selected_index as usize];
                    de.current_brush.tiles.push(tile_data.to_short());
                }
            }
        });
        ui.horizontal(|ui| {
            // Clear button
            if ui.button("Clear Brush").clicked() {
                log_write("Clearing current Brush", LogLevel::LOG);
                de.current_brush.clear();
            }
        });
    }
}

fn do_tile_draw(ui: &mut egui::Ui, brush: &mut Brush, palette: &[Palette;16], tiles: &Vec<u8>, col_mode: &u32, pal_offset: &u8) {
    let top_left: Pos2 = ui.min_rect().min;
    // First, draw the entire thing
    for y in 0..BRUSH_TILES_WIDE {
        for x in 0..BRUSH_TILES_WIDE {
            let painter: &Painter = ui.painter();
            let true_position: Pos2 = top_left + Vec2::new((x as f32) * BRUSH_TILE_DIM, (y as f32) * BRUSH_TILE_DIM);
            let rect: Rect = Rect::from_min_size(true_position, BRUSH_TILE_RECT);
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GRAY), egui::StrokeKind::Middle);
        }
        if y % 2 == 0 && y != 0 {
            let left_line_pos = top_left + Vec2::new(0.0, y as f32 * BRUSH_TILE_DIM);
            let right_line_pos = top_left + Vec2::new(16.0 * BRUSH_TILE_DIM, y as f32 * BRUSH_TILE_DIM);
            ui.painter().line(vec![left_line_pos,right_line_pos], Stroke::new(1.0, Color32::RED));
        }
    }
    for x in 0..16 {
        if x % 2 == 0 && x != 0 {
            let top_line_pos = top_left + Vec2::new(x as f32 * BRUSH_TILE_DIM, 0.0);
            let bottom_line_pos = top_left + Vec2::new(x as f32 * BRUSH_TILE_DIM, 16.0 * BRUSH_TILE_DIM);
            ui.painter().line(vec![top_line_pos,bottom_line_pos], Stroke::new(1.0, Color32::RED));
        }
    }
    // Then draw the tiles themselves
    for y in 0..brush.height {
        for x in 0..brush.width {
            let painter: &Painter = ui.painter();
            let true_position: Pos2 = top_left + Vec2::new((x as f32) * BRUSH_TILE_DIM, (y as f32) * BRUSH_TILE_DIM);
            let rect: Rect = Rect::from_min_size(true_position, BRUSH_TILE_RECT);
            let index: usize = (y as usize) * (brush.width as usize) + (x as usize);

            if index >= brush.tiles.len() {
                log_write(format!("Brush index is out of bounds, was {} but len is {}; calc'ed with x/y/brsw: {}/{}/{}",
                index,brush.tiles.len(),&x,&y,brush.width), LogLevel::ERROR);
            } else {
                // Do the actual tile draw
                if *col_mode == 0x0 {
                    let tile: MapTileRecordData = MapTileRecordData::new(&brush.tiles[index]);
                    // Check if out of bounds (subtract palette offset, +1 for universal palette)
                    let pal_id_signed = tile.palette_id as i32 + *pal_offset as i32 + 1;
                    if pal_id_signed < 0 || pal_id_signed >= 16 {
                        log_write(format!("Palette ID out of range: {}",pal_id_signed), LogLevel::ERROR);
                        continue;
                    }
                    if pal_id_signed as usize >= palette.len() {
                        log_write(format!("pal ID overflow in brush tile drawing: 0x{:X} >= 0x{:X}",pal_id_signed,palette.len()), LogLevel::ERROR);
                        return;
                    }
                    let cur_pal = &palette[pal_id_signed as usize];
                    let byte_array = &get_pixel_bytes_16(tiles, &tile.tile_id);
                    let nibble_array = pixel_byte_array_to_nibbles(byte_array);
                    let color_image = color_image_from_pal(cur_pal, &nibble_array);
                    let t = ui.ctx().load_texture("brushtile16", color_image, egui::TextureOptions::NEAREST);
                    let uvs = get_uvs_from_tile(&tile);
                    painter.image(t.id(), rect, uvs, Color32::WHITE);
                } else if *col_mode == 0x1 {
                    // 256 colors
                }
            }
        }
    }
}

#[cfg(test)]
mod tests_brushes {
    use super::*;

    #[test]
    fn test_parse() {
        let test_json_str = r#"
            {
                "tileset": "test_tiles",
                "name": "test brush",
                "width": 2,
                "height": 2,
                "palette_offset": 3,
                "tiles": [
                    1234,
                    4321,
                    1111,
                    2222
                ]
            }
        "#;
        let b: Brush = serde_json::from_str(test_json_str).expect("Brush should parse properly");
        assert_eq!(b.tileset,"test_tiles");
        assert_eq!(b.name,"test brush");
        assert_eq!(b.width,2);
        assert_eq!(b.height,2);
        assert_eq!(b.palette_offset,3);
        assert_eq!(b.tiles.len(),4);
        assert_eq!(b.tiles[0],1234);
        assert_eq!(b.tiles[3],2222);
    }

    #[test]
    #[should_panic]
    fn test_parse_failure() {
        let test_json_str = r#"
            {
                "tileset": "test_tiles",
                "name": "test brush",
                "width": 257,
                "height": 2,
                "palette_offset": 3,
                "tiles": [
                    1234,
                    4321,
                    1111,
                    2222
                ]
            }
        "#;
        let _b: Brush = serde_json::from_str(test_json_str).expect("Brush should parse properly");
    }
}
