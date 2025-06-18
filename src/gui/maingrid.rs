use std::f32::consts::PI;

use egui::{Align2, Color32, ColorImage, Context, FontId, Image, Painter, Pos2, Rect, Response, Stroke, Vec2};
use uuid::Uuid;

use crate::{data::{area::{AREA_RECT_COLOR, AREA_RECT_COLOR_SELECTED}, backgrounddata::BackgroundData, path::PathPoint, scendata::colz::{self, draw_collision}, sprites::{draw_sprite, LevelSprite}, types::{get_cached_texture, set_cached_texture, CurrentLayer, MapTileRecordData, Palette, TileCache}}, engine::displayengine::DisplayEngine, utils::{self, log_write, LogLevel}};

const TILE_WIDTH_PX: f32 = 8.0;
const TILE_HEIGHT_PX: f32 = 8.0;
const TILE_RECT: Vec2 = Vec2::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
const TILE_OUTER_PADDING: f32 = 10.0;
const RECT_TRIM_PADDING_TILE: f32 = 1.0;
const SPRITE_RECT: Vec2 = Vec2::new(TILE_WIDTH_PX * 2.0, TILE_HEIGHT_PX * 2.0);
const SPRITE_BG_COLOR: Color32 = Color32::from_rgba_premultiplied(0xff, 0x00, 0xff, 0x40);
const SPRITE_BG_COLOR_SELECTED: Color32 = Color32::from_rgba_premultiplied(0x00, 0xff, 0x00, 0xff);
const FONT: FontId = FontId { size: 12.0, family: egui::FontFamily::Monospace };
const BG_SELECTION_FILL: Color32 = Color32::from_rgba_premultiplied(0x80, 0x65, 0xb5, 0xA0);
const BG_SELECTION_FILL_INVERT: Color32 = Color32::from_rgba_premultiplied(0x65, 0x80, 0xb5, 0xA0);
const BG_SELECTION_STROKE: Color32 = Color32::WHITE;

/// Active drawing for various visible data layers
/// 
/// Each one takes in the display data plus a UI reference, then combines the two
/// to create a drawn layer. This also includes logic to disable drawing the layer.
pub fn render_primary_grid(ui: &mut egui::Ui, de: &mut DisplayEngine, vrect: &Rect) {
    puffin::profile_function!();
    draw_background(ui, de, vrect, 3, de.display_settings.show_bg3);
    draw_background(ui, de, vrect, 2, de.display_settings.show_bg2);
    draw_background(ui, de, vrect, 1, de.display_settings.show_bg1);
    if de.display_settings.show_breakable_rock {
        draw_breakable_rock(ui, de);
    }
    if de.display_settings.show_sprites {
        draw_sprites(ui, de, vrect);
    }
    if de.display_settings.show_col { // Goes over Sprites since some work with collision
        draw_collision_layer(ui, de, vrect);
    }
    if de.display_settings.show_paths {
        draw_paths(ui, de);
    }
    if de.display_settings.show_entrances {
        draw_entrances(ui, de);
    }
    if de.display_settings.show_exits {
        draw_exits(ui, de);
    }
    if de.display_settings.show_triggers {
        draw_triggers(ui, de);
    }
}

fn draw_collision_layer(ui: &mut egui::Ui, de: &mut DisplayEngine,vrect: &Rect) {
    puffin::profile_function!();
    let Some(bg_with_col) = de.loaded_map.get_bg_with_colz() else { return };
    let Some(bg) = de.loaded_map.get_background(bg_with_col) else { return };
    let Some(info_c) = bg.get_info() else { return };
    let grid_width = info_c.layer_width as u32;
    let Some(col) = bg.get_colz_mut() else { return };
    // Precursors
    let true_rect = ui.min_rect();
    let top_left: Pos2 = ui.min_rect().min;
    // These will be used for rendering fewer tiles to save CPU
    let leftmost_tile = vrect.left() / TILE_WIDTH_PX;
    let rightmost_tile = vrect.right() / TILE_WIDTH_PX;
    let uppermost_tile = vrect.top() / TILE_HEIGHT_PX;
    let bottommost_tile = vrect.bottom() / TILE_HEIGHT_PX;
    // Start!
    let mut col_index: u32 = 0;
    // Include the image cached, and tint it light blue to show it's different
    let image: Image<'_> = egui::Image::new(egui::include_image!("../../assets/collision_coin.png")).tint(Color32::LIGHT_BLUE);
    for col_u8 in &mut col.col_tiles {
        if *col_u8 != 0 { // 0x0 = Nothing, skip render
            let painter: &Painter = ui.painter();
            let tile_x: f32 = (col_index % (grid_width/2)) as f32;
            let tile_y: f32 = (col_index / (grid_width/2)) as f32;
            // Don't render outside the viewport
            if tile_x > rightmost_tile/2.0 + RECT_TRIM_PADDING_TILE {
                // Skip
                col_index += 1;
                continue;
            }
            if tile_x < leftmost_tile/2.0 - RECT_TRIM_PADDING_TILE {
                // Skip
                col_index += 1;
                continue;
            }
            if tile_y > bottommost_tile/2.0 + RECT_TRIM_PADDING_TILE {
                // Skip
                col_index += 1;
                continue;
            }
            if tile_y < uppermost_tile/2.0 - RECT_TRIM_PADDING_TILE {
                // Skip
                col_index += 1;
                continue;
            }
            let tile_x_px: f32 = tile_x * (TILE_WIDTH_PX*2.0);
            let tile_y_px: f32 = tile_y * (TILE_HEIGHT_PX*2.0);
            let rect: Rect = Rect::from_min_size(top_left + Vec2::new(tile_x_px, tile_y_px), colz::COLLISION_SQUARE);
            let col_bg_color = colz::COLLISION_BG_COLOR;
            if *col_u8 == 0x1 { // Square, 95% of non-empty colliders (I checked)
                painter.rect_filled(rect, 0.0, col_bg_color);
                painter.rect_stroke(rect, 0.0, Stroke::new(1.0, colz::COLLISION_OUTLINE_COLOR), egui::StrokeKind::Middle);
            } else if *col_u8 == 0x1A { // 0x1A is the Collision coin
                image.paint_at(ui, rect);
            } else {
                draw_collision(painter, &rect, *col_u8);
            }
            // If it overlaps the deletion rectangle... delete it
            if
                *col_u8 != 0x00
                && de.col_selector_status.delete_under
                && de.col_selector_status.selecting_rect.intersects(rect)
            {
                //let _ = de.loaded_map.set_col_tile(bg_with_col, col_index as u16, 0x00);
                *col_u8 = 0x00;
                de.graphics_update_needed = true;
                de.unsaved_changes = true;
            }
        }
        col_index += 1;
    }
    if de.col_selector_status.delete_under {
        // Now that it deleted what it should, disable it all
        de.col_selector_status.delete_under = false;
        de.col_selector_status.dragging = false;
        de.col_selector_status.selecting_rect = Rect::NOTHING;
    }
    // COLZ Interactivity //
    if de.display_settings.current_layer == CurrentLayer::Collision {
        let col_sense_resp: Response = ui.interact(true_rect, egui::Id::new("col_tile_click"), egui::Sense::all());
        // Do it in three separate ones to avoid repeated input checking that won't be used
        if col_sense_resp.clicked() {
            // Add a new tile 
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - true_rect.min;
                let tile_index = local_pos_to_col_index(&local_pos, grid_width);
                if tile_index as usize >= col.col_tiles.len() {
                    log_write(format!("Index out of bounds: {} >= {}",tile_index,col.col_tiles.len()), LogLevel::Error);
                    return;
                }
                de.loaded_map.set_col_tile(bg_with_col, tile_index as u16, de.col_tile_to_place);
                de.graphics_update_needed = true;
                de.unsaved_changes = true;
            }
        } else if col_sense_resp.secondary_clicked() {
            // Clear the tile
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - true_rect.min;
                let tile_index = local_pos_to_col_index(&local_pos, grid_width);
                if tile_index as usize >= col.col_tiles.len() {
                    log_write(format!("Index out of bounds: {} >= {}",tile_index,col.col_tiles.len()), LogLevel::Error);
                    return;
                }
                // 0x00 is empty
                de.loaded_map.set_col_tile(bg_with_col, tile_index as u16, 0x00);
                de.graphics_update_needed = true;
                de.unsaved_changes = true;
            }
        } else if col_sense_resp.middle_clicked() {
            // Copy the tile (and show info)
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - true_rect.min;
                let tile_index = local_pos_to_col_index(&local_pos, grid_width);
                if tile_index as usize >= col.col_tiles.len() {
                    log_write(format!("Index out of bounds: {} >= {}",tile_index,col.col_tiles.len()), LogLevel::Error);
                    return;
                }
                let tile = &col.col_tiles[tile_index as usize];
                // Don't copy empty ones, that's for right clicking
                if *tile != 0x00 {
                    log_write(format!("Copied tile of type '0x{:X}' at index 0x{:X}",tile,tile_index), LogLevel::Log);
                    de.col_tile_to_place = *tile; // Copies it in
                }
            }
        }
        if col_sense_resp.drag_started() {
            // Only right click drag
            if !ui.input(|i| i.pointer.secondary_down()) {
                return;
            }
            de.col_selector_status.dragging = true;
            let cur_pos: Pos2 = ui.ctx().pointer_interact_pos().expect("Failed to get pointer interaction position");
            de.col_selector_status.start_pos = cur_pos;
            de.col_selector_status.end_pos = cur_pos; // Starts as empty square
        }
        if col_sense_resp.dragged() {
            // Only right click drag
            if !ui.input(|i| i.pointer.secondary_down()) {
                return;
            }
            let Some(cur_pos) = ui.ctx().pointer_interact_pos() else {
                log_write("Failed to get pointer_interact_pos in col .dragged", LogLevel::Error);
                return;
            };
            de.col_selector_status.end_pos = cur_pos;
            // Draw
            let drag_rect: Rect = Rect::from_two_pos(de.col_selector_status.start_pos, de.col_selector_status.end_pos);
            ui.painter().rect_filled(drag_rect, 0.0, BG_SELECTION_FILL);
            // Store
            de.col_selector_status.selecting_rect = drag_rect;
        }
        if col_sense_resp.drag_stopped() {
            de.col_selector_status.dragging = false;
            de.col_selector_status.start_pos = Pos2::new(0.0, 0.0);
            de.col_selector_status.end_pos = Pos2::new(0.0, 0.0);
            de.col_selector_status.delete_under = true;
            // Set this once deletion done, so you can do the deletions
            //de.col_selector_status.selecting_rect = Rect::NOTHING;
        }
    }
}

fn draw_triggers(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    let top_left_screen: Pos2 = ui.min_rect().min;
    let Some(area) = de.loaded_map.get_area() else { return };
    for trigger in &area.triggers {
        let rect = trigger.get_rect(top_left_screen, TILE_WIDTH_PX, TILE_HEIGHT_PX);
        if de.trigger_settings.selected_uuid == trigger.uuid {
            ui.painter().rect_filled(rect, 0.0, AREA_RECT_COLOR_SELECTED);
        } else {
            ui.painter().rect_filled(rect, 0.0, AREA_RECT_COLOR);
        }
    }

    if de.display_settings.current_layer == CurrentLayer::Triggers {
        let click_response = ui.interact(ui.min_rect(), egui::Id::new("AREA_click"), egui::Sense::click());
        if click_response.clicked() {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let mut found: bool = false;
                for trigger in &area.triggers {
                    let rect = trigger.get_rect(top_left_screen, TILE_WIDTH_PX, TILE_HEIGHT_PX);
                    if rect.contains(pointer_pos) {
                        // UUID is copyable
                        de.trigger_settings.selected_uuid = trigger.uuid;
                        found = true;
                    }
                }
                if !found {
                    de.trigger_settings.selected_uuid = Uuid::nil();
                }
            }
        }
    }
}

fn draw_breakable_rock(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    let top_left_screen: Pos2 = ui.min_rect().min;
    // It's read-only, we aren't changing it
    let Some(blkz_data) = de.loaded_map.get_blkz().cloned() else { return };

    let Some(colz_layer) = de.loaded_map.get_bg_with_colz() else {
        log_write("There is no layer with COLZ", LogLevel::Error);
        return;
    };
    let bg = de.loaded_map.get_background(colz_layer).expect("Already confirmed COLZ");
    let info = bg.get_info().expect("Info guaranteed on BGs for rock");

    let mut tile_index: i32 = 0;
    let base_offset_x = blkz_data.x_offset as f32;
    let base_offsey_y = blkz_data.y_offset as f32;
    for tile in &blkz_data.tiles {
        let tile_x_rect_offset: f32 = (tile_index % blkz_data.width as i32) as f32;
        let tile_y_rect_offset: f32 = (tile_index / blkz_data.width as i32) as f32;
        let tile_x_offset = tile_x_rect_offset + base_offset_x;
        let tile_y_offset = tile_y_rect_offset + base_offsey_y;
        let screen_x = tile_x_offset * TILE_WIDTH_PX;
        let screen_y = tile_y_offset * TILE_HEIGHT_PX;
        let true_rect: Rect = Rect::from_min_size(top_left_screen + Vec2::new(screen_x, screen_y), TILE_RECT);
        let render_pal_id = tile.get_render_pal_id(bg._pal_offset, info.color_mode);
        if render_pal_id >= 16 {
            log_write(format!("palette id for render too high in draw_breakable_rock: {}", render_pal_id), LogLevel::Error);
            continue;
        }
        let palette = &de.bg_palettes[render_pal_id];
        let pixel_tiles = bg.pixel_tiles_preview.as_ref().expect("There should be pixel tiles on the background with COLZ");
        draw_blkz_tile(tile, palette, pixel_tiles, &true_rect,ui.ctx(),ui.painter());
        // Placement is good!
        //ui.painter().rect_filled(true_rect, 0.0, Color32::RED);
        tile_index += 1;
    }
}

fn draw_blkz_tile(
    tile: &MapTileRecordData, palette: &Palette,
    pixel_tiles: &[u8], true_rect: &Rect,
    ctx: &Context, painter: &Painter
) {
    let byte_array = &utils::get_pixel_bytes_16(pixel_tiles, &tile.tile_id);
    let nibble_array = utils::pixel_byte_array_to_nibbles(byte_array);
    let color_image = utils::color_image_from_pal(palette, &nibble_array);
    let handle = ctx.load_texture("tile16", color_image, egui::TextureOptions::NEAREST);
    let uvs = utils::get_uvs_from_tile(tile);
    painter.image(handle.id(), *true_rect, uvs, Color32::WHITE);
}

fn draw_entrances(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    let top_left: Pos2 = ui.min_rect().min;
    let Some(map_index) = de.map_index else { return };
    let maps_count = de.loaded_course.level_map_data.len();
    if map_index >= maps_count {
        // This will cause an overflow panic
        log_write(format!(
            "Map Index is somehow greater than level map data length ({} >= {})",
            &map_index,&maps_count), LogLevel::Fatal);
        return;
    }
    let entrances = &de.loaded_course.level_map_data[map_index].map_entrances;
    for entrance in entrances {
        let x_no_offset = (entrance.entrance_x as f32) * TILE_WIDTH_PX;
        let y_no_offset = (entrance.entrance_y as f32) * TILE_HEIGHT_PX;
        let true_pos: Pos2 = top_left + Vec2::new(x_no_offset, y_no_offset);
        let rect = Rect::from_min_size(true_pos, SPRITE_RECT);

        if entrance.uuid == de.course_settings.selected_entrance.unwrap_or(Uuid::nil()) {
            ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(0x00, 0xff, 0, 0xA0));
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Middle);
        } else {
            ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(0x00, 0xff, 0, 0x40));
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Middle);
        }
    }
}

fn draw_exits(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    let top_left: Pos2 = ui.min_rect().min;
    let Some(map_index) = de.map_index else { return };
    let maps_count = de.loaded_course.level_map_data.len();
    if map_index >= maps_count {
        // This will cause an overflow panic if not stopped
        log_write(format!(
            "Map Index is somehow greater than level map data length ({} >= {})",
            &map_index,&maps_count), LogLevel::Fatal);
        return;
    }
    let exits = &de.loaded_course.level_map_data[map_index].map_exits;
    for exit in exits {
        let x_no_offset = (exit.exit_x as f32) * TILE_WIDTH_PX;
        let y_no_offset = (exit.exit_y as f32) * TILE_HEIGHT_PX;
        let true_pos: Pos2 = top_left + Vec2::new(x_no_offset, y_no_offset);
        let rect = Rect::from_min_size(true_pos, SPRITE_RECT);
        if exit.uuid == de.course_settings.selected_exit.unwrap_or(Uuid::nil()) {
            ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(0xff, 0, 0, 0xA0));
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Middle);
        } else {
            ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(0xff, 0, 0, 0x40));
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Middle);
        }
    }
}

const PATH_SELECTION_DISTANCE: f32 = 20.0;

fn draw_paths(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    puffin::profile_function!();
    let arm9 = de.loaded_arm9.as_ref().expect("ARM9 must exist");
    let top_left: Pos2 = ui.min_rect().min;
    if let Some(path_database) = &de.path_data {
        for line in &path_database.lines {
            let mut line_points: Vec<Pos2> = Vec::new();
            let path_selected = de.path_settings.selected_line == line.uuid;
            for point in &line.points {
                let placement_vec: Vec2 = Vec2::new(
                    ((point.x_fine >> 15) as f32) * TILE_WIDTH_PX,
                    ((point.y_fine >> 15) as f32) * TILE_HEIGHT_PX
                );
                let true_pos: Pos2 = top_left + placement_vec;
                line_points.push(true_pos);
                let rect = Rect::from_min_size(true_pos, Vec2 { x: 6.0, y: 6.0 });
                let point_selected = de.path_settings.selected_point == point.uuid;
                if point_selected {
                    ui.painter().rect_filled(rect, 0.0, Color32::ORANGE);
                }
                ui.painter().rect_stroke(rect, 0.0,
                    Stroke::new(1.0,
                        if path_selected { Color32::LIGHT_RED } else { Color32::RED }
                    ),
                    egui::StrokeKind::Outside
                );
                if point.distance >= 0 && point.distance != 0 {
                    let test_val = utils::get_sin_cos_table_value(arm9, point.angle as u16,de.game_version);
                    let x_offset = ((test_val.x as i32) * (point.distance as i32)) >> 12; // Note: this includes the tile width
                    let y_offset = ((test_val.y as i32) * (point.distance as i32)) >> 12; // This will need changing once zoom is added
                    //println!("test_val: {:?}", test_val);
                    let end_pos: Pos2 = Pos2::new(true_pos.x + (x_offset as f32), true_pos.y + (y_offset as f32));//
                    let stroke = Stroke::new(
                        if point_selected { 2.0 } else { 1.0 },
                        if point_selected {Color32::GREEN} else { Color32::RED }
                    );
                    ui.painter().line(vec![true_pos,end_pos], stroke);
                } else {
                    // Point distance is negative
                    // Calculations done here: 02054b34
                }
            }
            // Circles
            for (i, cur_point) in line.points.iter().enumerate() {
                if i == line.points.len() - 1 {
                    // Skip the last one
                    break;
                }
                if cur_point.distance >= 0 {
                    // It's a straight line, skip
                    continue;
                }
                // Copy
                let next_point: PathPoint = line.points[i+1];
                let (circle_point_fine,radius,rads) = utils::get_curve_fine(cur_point, &next_point);
                let circle_radius = (radius >> 12) as f32;
                let circle_vec: Vec2 = Vec2::new(
                    ((circle_point_fine.x as u32 >> 15) as f32) * TILE_WIDTH_PX,
                    ((circle_point_fine.y as u32 >> 15) as f32) * TILE_HEIGHT_PX
                );
                let circle_pos: Pos2 = top_left + circle_vec;
                let point_selected = de.path_settings.selected_point == cur_point.uuid;
                // This is the general circle
                //ui.painter().circle_stroke(circle_pos, circle_radius, egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xff, 0, 0, 0x05)));
                
                let circle_stroke = egui::Stroke::new(if point_selected { 2.0 } else { 1.0 },
                if point_selected {
                    Color32::GREEN
                } else {
                    Color32::from_rgba_unmultiplied(0xff, 0, 0, 0x55)
                });
                let segments: usize = 5;
                let mut points: Vec<Pos2> = vec![];
                const RAD_UNIT: f32 = PI / 2.0; // 90 degrees in Radians
                for i in 0..=segments {
                    // Divide into segments, then do radian offset
                    let angle = ((i as f32) / (segments as f32) * RAD_UNIT)+rads;
                    let x = circle_pos.x + circle_radius * angle.cos();
                    let y = circle_pos.y - circle_radius * angle.sin();
                    points.push(Pos2 { x, y });
                }
                ui.painter().add(egui::Shape::line(points, circle_stroke));
            }
        }
        // Interactivity
        if de.display_settings.current_layer == CurrentLayer::Paths {
            let click_response = ui.interact(ui.min_rect(), egui::Id::new("PATH_click"), egui::Sense::click());
            if click_response.clicked() {
                if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                    let local_pos = pointer_pos - ui.min_rect().min;
                    let mut closest_uuid: Uuid = Uuid::nil();
                    let mut closest_line_uuid: Uuid = Uuid::nil();
                    let mut shortest_distance: f32 = 999999999.0;
                    for line in &path_database.lines {
                        for point in &line.points {
                            let placement_vec: Vec2 = Vec2::new(
                                ((point.x_fine >> 15) as f32) * TILE_WIDTH_PX,
                                ((point.y_fine >> 15) as f32) * TILE_HEIGHT_PX
                            );
                            let point_pos = placement_vec.to_pos2();
                            let distance = utils::distance(point_pos, local_pos.to_pos2());
                            if distance < shortest_distance {
                                shortest_distance = distance;
                                closest_uuid = point.uuid;
                                closest_line_uuid = line.uuid;
                            }
                        }
                    }
                    if shortest_distance <= PATH_SELECTION_DISTANCE {
                        de.path_settings.selected_point = closest_uuid;
                        de.path_settings.selected_line = closest_line_uuid;
                    }
                }
            }
            if click_response.secondary_clicked() && !de.path_settings.selected_line.is_nil() {
                if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                    let local_pos = pointer_pos - ui.min_rect().min;
                    let x_fine = ((local_pos.x / TILE_WIDTH_PX) as u32) << 15;
                    let y_fine = ((local_pos.y / TILE_HEIGHT_PX) as u32) << 15;
                    // You are adding it on the end, therefore distance defaults to 0
                    let p = PathPoint::new(0, 0, x_fine, y_fine);
                    let puuid = p.uuid; // Copies
                    let Some(path_data) = de.loaded_map.get_path() else {
                        log_write("Failed to get PathDatabase", LogLevel::Error);
                        return;
                    };
                    let line = path_data.lines.iter_mut().find(|x| x.uuid == de.path_settings.selected_line);
                    if let Some(l) = line {
                        l.points.push(p);
                        de.path_settings.selected_point = puuid;
                        de.graphics_update_needed = true;
                        de.unsaved_changes = true;
                    } else {
                        log_write("Failed to get PathLine for new PathPoint", LogLevel::Error);
                    }
                }
            }
        }
    }
}

fn draw_sprites(ui: &mut egui::Ui, de: &mut DisplayEngine, vrect: &Rect) {
    puffin::profile_function!();
    let top_left: Pos2 = ui.min_rect().min;
    let mut update_map: bool = false;
    // If this always fires, it will block COLZ clicks
    let mut click_fallback_response: Option<Response> = Option::None;
    if de.display_settings.current_layer == CurrentLayer::Sprites {
        click_fallback_response = Some(ui.interact(ui.min_rect(), egui::Id::new("sprite_click_fallback"), egui::Sense::click()));
    }
    // It's one way, don't mutable borrow
    let sprite_list: Vec<LevelSprite> = de.level_sprites.clone();
    for level_sprite in sprite_list {
        if level_sprite.x_position == 0xffff && level_sprite.y_position == 0xffff {
            let leftmost_tile = vrect.left() / TILE_WIDTH_PX;
            let uppermost_tile = vrect.top() / TILE_HEIGHT_PX;
            de.loaded_map.move_sprite(level_sprite.uuid, leftmost_tile as u16 + 2, uppermost_tile as u16 + 2);
            de.graphics_update_needed = true;
            // Cancel the update drawing
            return;
        }
        let placement_vec: Vec2 = Vec2::new(
            (level_sprite.x_position as f32) * TILE_WIDTH_PX,
            (level_sprite.y_position as f32) * TILE_HEIGHT_PX
        );
        let true_pos: Pos2 = top_left + placement_vec;
        let rect = Rect::from_min_size(true_pos, SPRITE_RECT);

        let mut drawn_rects = draw_sprite(
            ui, &rect, &level_sprite, de,8.0,
            de.selected_sprite_uuids.contains(&level_sprite.uuid)
        );
        // No render for it, do square (or do it anyway)
        if drawn_rects.is_empty() || de.display_settings.show_box_for_rendered {
            // We want the source rect to be clickable too
            drawn_rects.push(rect);

            if de.selected_sprite_uuids.contains(&level_sprite.uuid) {
                ui.painter().rect_filled(rect, 0.0, SPRITE_BG_COLOR_SELECTED);
            } else {
                ui.painter().rect_filled(rect, 0.0, SPRITE_BG_COLOR);
            }
            ui.painter().text(
                true_pos, Align2::LEFT_TOP,
                format!("{:02X}",level_sprite.object_id),
                FONT, Color32::WHITE
            );
        }

        // Interactivity
        if de.display_settings.current_layer == CurrentLayer::Sprites {
            let is_shift = ui.ctx().input(|i| i.modifiers.shift);
            for (i,r) in drawn_rects.iter().enumerate() {
                let click_response = ui.interact(*r, egui::Id::new(format!("sprite_click_{}_{}",level_sprite.uuid,i)), egui::Sense::click());
                if click_response.clicked() {
                    if is_shift {
                        de.selected_sprite_uuids.push(level_sprite.uuid); // UUID derives Copy
                    } else {
                        de.selected_sprite_uuids.clear();
                        de.selected_sprite_uuids.push(level_sprite.uuid); // UUID derives Copy
                    }
                    // Remove duplicates
                    de.selected_sprite_uuids.dedup();
                    // If length is one, handle gui
                    if de.selected_sprite_uuids.len() == 1 {
                        de.latest_sprite_settings = utils::bytes_to_hex_string(&level_sprite.settings);
                    }
                }
                // Debug
                if click_response.middle_clicked() {
                    println!("== Middle Clicked Sprite {} ==",level_sprite.uuid);
                    println!("- {}",level_sprite);
                }
                // If selected
                if de.selected_sprite_uuids.contains(&level_sprite.uuid) {
                    // Hover is a grab icon
                    let interaction_id = egui::Id::new(format!("sprite_hover_{}_{}",level_sprite.uuid,i));
                    let interaction = ui.interact(*r, interaction_id, egui::Sense::all());
                    if interaction.hovered() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
                    }
                    // Drag logic
                    if interaction.drag_started() {
                        log_write("Started dragging sprite", LogLevel::Debug);
                        de.sprite_drag_status.dragging_uuid = level_sprite.uuid; // Implements copy
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Move);
                        let cur_pos = ui.ctx().pointer_interact_pos().expect("Failed to get pointer interaction position");
                        de.sprite_drag_status.start_x = cur_pos.x;
                        de.sprite_drag_status.start_y = cur_pos.y;
                    }
                    if interaction.dragged() {
                        //println!("Drag moving");
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Move);
                        let cur_pos = ui.ctx().pointer_interact_pos().expect("Failed to get dragged cursor");
                        let preview_rect = Rect::from_min_size(cur_pos, SPRITE_RECT);
                        ui.painter().rect_filled(preview_rect, 0.0, SPRITE_BG_COLOR_SELECTED);
                    }
                    if interaction.drag_stopped() {
                        //println!("Drag stopped");
                        de.sprite_drag_status.dragging_uuid = Uuid::nil();
                        let latest_pos: Pos2 = ui.ctx().pointer_interact_pos().expect("CTX should hold pointer interaction position");
                        let drag_stop_pos: Vec2 = latest_pos.to_vec2() - top_left.to_vec2();
                        // 0.5 makes it round to nearest when slicing off the precision
                        let true_new_x: u16 = ((drag_stop_pos.x + 0.5) / TILE_WIDTH_PX) as u16;
                        let true_new_y: u16 = ((drag_stop_pos.y + 0.5) / TILE_HEIGHT_PX) as u16;
                        de.sprite_drag_status.start_x = 0.0;
                        de.sprite_drag_status.start_y = 0.0;
                        let og_sprite_tile_x = level_sprite.x_position as i32;
                        let og_sprite_tile_y = level_sprite.y_position as i32;
                        let x_tile_movement = (true_new_x as i32) - og_sprite_tile_x;
                        let y_tile_movement = (true_new_y as i32) - og_sprite_tile_y;
                        for selspr in &de.selected_sprite_uuids {
                            let Some(og_sprite_data) = de.get_loaded_sprite_by_uuid(selspr) else {
                                log_write(format!("Sprite Uuid '{}' not found when moving",selspr), LogLevel::Error);
                                continue;
                            };
                            let mut move_to_x = og_sprite_data.x_position as i32 + x_tile_movement;
                            if move_to_x < 0 {
                                move_to_x = 0;
                            }
                            let mut move_to_y = og_sprite_data.y_position as i32 + y_tile_movement;
                            if move_to_y < 0 {
                                move_to_y = 0;
                            }
                            de.loaded_map.move_sprite(*selspr, move_to_x as u16, move_to_y as u16);
                        }
                        de.unsaved_changes = true;
                        update_map = true;
                    }
                }
            }
        }
    }
    // Fallback/background/placement (not existing)
    if de.display_settings.current_layer == CurrentLayer::Sprites {
        if let Some(cfr) = &click_fallback_response {
            if cfr.clicked() { // Clicked on empty background
                de.selected_sprite_uuids.clear();
            }
            if cfr.secondary_clicked() { // Right clicked on empty background = place
                log_write("Placing new sprite from right click...", LogLevel::Debug);
                // Retrieve the base sprite ID to create, usually set by Add Sprite
                let Some(new_sprite_id) = de.selected_sprite_to_place else {
                    log_write("Could not place sprite, none selected to add", LogLevel::Debug);
                    return;
                };
                if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                    let local_pos = pointer_pos - ui.min_rect().min;
                    let base_tile_x: u16 = (local_pos.x/TILE_WIDTH_PX) as u16;
                    let base_tile_y: u16 = (local_pos.y/TILE_HEIGHT_PX) as u16;
                    let new_uuid = de.loaded_map.add_new_sprite_at(new_sprite_id, base_tile_x, base_tile_y);
                    log_write(format!("Placed sprite with UUID {new_uuid}"), LogLevel::Debug);
                    de.selected_sprite_uuids = vec![new_uuid]; // Select only it
                    de.unsaved_changes = true;
                    update_map = true;
                } else {
                    log_write("Could not get pointer pos when right clicking Sprite", LogLevel::Error);
                }
            }
        }
    }
    if update_map {
        de.graphics_update_needed = true;
    }
}

fn draw_background(
    ui: &mut egui::Ui, de: &mut DisplayEngine,
    vrect: &Rect, whichbg: u8,
    show: bool
) {
    puffin::profile_function!();
    // These will be used for rendering fewer tiles to save CPU
    let leftmost_tile = vrect.left() / TILE_WIDTH_PX;
    let rightmost_tile = vrect.right() / TILE_WIDTH_PX;
    let uppermost_tile = vrect.top() / TILE_HEIGHT_PX;
    let bottommost_tile = vrect.bottom() / TILE_HEIGHT_PX;
    #[allow(unused_assignments)] // Unknown why this is needed
    let mut bg_layer_opt: Option<&BackgroundData> = Option::None;
    #[allow(unused_assignments)] // Same here
    let mut tc: Option<&mut TileCache> = Option::None;
    match whichbg {
        1 => {
            bg_layer_opt = de.bg_layer_1.as_ref();
            tc = Some(&mut de.tile_cache_bg1);
        }
        2 => {
            bg_layer_opt = de.bg_layer_2.as_ref();
            tc = Some(&mut de.tile_cache_bg2);
        }
        3 => {
            bg_layer_opt = de.bg_layer_3.as_ref();
            tc = Some(&mut de.tile_cache_bg3);
        }
        _ => {
            log_write(format!("Unusual whichbg value in draw_background: '{}'",whichbg), LogLevel::Error);
            return;
        }
    }
    if let Some(layer) = bg_layer_opt {
        let info = layer.get_info().expect("INFO is guaranteed in SCENs");
        let is_selected_layer: bool = (de.display_settings.current_layer as u8) == whichbg;
        let grid_width: u32 = info.layer_width as u32;
        let grid_height_px = (info.layer_height as f32) * TILE_HEIGHT_PX + TILE_OUTER_PADDING;
        let grid_width_px = (grid_width as f32)*TILE_WIDTH_PX + TILE_OUTER_PADDING;
        let _ = ui.allocate_space(egui::vec2(grid_width_px, grid_height_px));
        if !show { // We still want the biggest one's space to show
            // But not RENDER. Just fill the space
            return;
        }
        let mut true_grid_rect = ui.min_rect();
        if info.x_offset_px != 0 || info.y_offset_px != 0 {
            true_grid_rect = true_grid_rect.translate(Vec2::new((info.x_offset_px * -1) as f32, (info.y_offset_px * -1) as f32));
        }
        let mut temp_selected_indexes: Vec<u32> = Vec::new();
        // MAP TILES //
        if let Some(map_tiles) = layer.get_mpbz() {
            if let Some(pixel_tiles) = &layer.pixel_tiles_preview {
                // Start cycle of each map tile
                let mut map_index: u32 = 0;
                let ctx = ui.ctx();
                let painter = ui.painter();
                for map_tile in &map_tiles.tiles {
                    let tile_x: f32 = (map_index % grid_width) as f32;
                    let tile_y: f32 = (map_index / grid_width) as f32;
                    // The following four checks prevented rendering ALL tiles
                    // Save some CPU
                    if tile_x > rightmost_tile + RECT_TRIM_PADDING_TILE {
                        // Skip
                        map_index += 1;
                        continue;
                    }
                    if tile_x < leftmost_tile - RECT_TRIM_PADDING_TILE {
                        // Skip
                        map_index += 1;
                        continue;
                    }
                    if tile_y > bottommost_tile + RECT_TRIM_PADDING_TILE {
                        // Skip
                        map_index += 1;
                        continue;
                    }
                    if tile_y < uppermost_tile - RECT_TRIM_PADDING_TILE {
                        // Skip
                        map_index += 1;
                        continue;
                    }
                    let tile_x_px: f32 = tile_x * TILE_WIDTH_PX;
                    let tile_y_px: f32 = tile_y * TILE_HEIGHT_PX;
                    let pal_id = map_tile.get_render_pal_id(layer._pal_offset, info.color_mode);
                    if pal_id >= 16 {
                        log_write(format!("Palette ID was too high when attempting to draw tile on bg {} (was 0x{:X})",whichbg,pal_id), LogLevel::Error);
                        log_write(format!("Offending MapTile data: {}",map_tile), LogLevel::Log);
                        // skip
                        map_index += 1;
                        continue;
                    }
                    let cur_pal = &de.bg_palettes[pal_id];
                    // Check if the tile id is out of bounds (Often caused by missing ANMZ data)
                    if map_tile.tile_id as usize >= pixel_tiles.len() {
                        log_write(format!("Tile ID was too high when attempting to draw tile on bg {} (was 0x{:X})",whichbg,map_tile.tile_id), LogLevel::Error);
                        log_write(format!("Offending MapTile data: {}",map_tile), LogLevel::Log);
                        // skip
                        map_index += 1;
                        continue;
                    }
                    // This is the actual rectangle the tile will be rendered in
                    let true_tile_rect: Rect = Rect::from_min_size(true_grid_rect.min + Vec2::new(tile_x_px, tile_y_px), TILE_RECT);
                    let mut selected: bool = false;
                    if de.bg_sel_data.selecting_rect.intersects(true_tile_rect) && is_selected_layer {
                        selected = true;
                        // Add to temporary index
                        if !temp_selected_indexes.contains(&map_index) {
                            temp_selected_indexes.push(map_index);
                        }
                    }
                    // Check if its in the real selection
                    if is_selected_layer && de.bg_sel_data.selected_map_indexes.contains(&map_index) {
                        selected = true;
                    }
                    let is_cur_lay_bg = de.display_settings.is_cur_layer_bg();
                    let dim = (!is_selected_layer && is_cur_lay_bg) || de.display_settings.current_layer == CurrentLayer::Collision;
                    if let Some(tilecache) = &mut tc {
                        if !info.is_256_colorpal_mode() {
                            draw_tile_16(
                                map_tile, cur_pal, ctx, pixel_tiles,
                                painter, tilecache,
                                &true_tile_rect, selected,dim);
                        } else if let Some(pltb) = layer.get_pltb() {
                            if pltb.palettes.is_empty() {
                                log_write("PLTB palettes were empty when trying to draw 256 tile!".to_owned(), LogLevel::Error);
                            } else {
                                draw_tile_256(
                                    map_tile, &pltb.palettes[0], ctx,
                                    pixel_tiles, painter, tilecache,
                                    &true_tile_rect, selected, dim);
                            }
                        } else {
                            log_write(format!("Failed to find PLTB data for tile drawing on bg '{}'",info.which_bg), LogLevel::Error);
                        }
                        
                    }
                    // Draw lines to show true edges of layers //
                    if tile_y as u32 == info.layer_height as u32 - 1 {
                        // True rect is the bottommost tile
                        let point_1 = true_tile_rect.left_bottom() + Vec2::new(1.0, 1.0);
                        let point_2 = true_tile_rect.right_bottom() + Vec2::new(-1.0, 1.0);
                        ui.painter().line(vec![point_1,point_2], egui::Stroke::new(1.0, if is_selected_layer {
                            Color32::RED
                        } else {
                            Color32::BLUE
                        }));
                    }
                    if tile_x as u32 == info.layer_width as u32 - 1 {
                        // True rect is the rightmost tile
                        let point_1 = true_tile_rect.right_top() + Vec2::new(1.0, 1.0);
                        let point_2 = true_tile_rect.right_bottom() + Vec2::new(1.0, -1.0);
                        ui.painter().line(vec![point_1,point_2], egui::Stroke::new(1.0, if is_selected_layer {
                            Color32::RED
                        } else {
                            Color32::BLUE
                        }));
                    }

                    map_index += 1;
                }
                // Interactivity //
                if is_selected_layer {
                    let interaction_id = egui::Id::new(format!("map_tile_interact_{}",whichbg));
                    // all() because it uses click, drag, and hover
                    let bg_interaction = ui.interact(true_grid_rect, interaction_id, egui::Sense::all());
                    if bg_interaction.drag_started() {
                        log_write("Started dragging in BG render function", LogLevel::Debug);
                        de.bg_sel_data.dragging = true;
                        let Some(cur_pos) = ui.ctx().pointer_interact_pos() else {
                            // This has failed before, somehow, so don't panic
                            log_write("Failed to get pointer_interact_pos in BG .drag_started", LogLevel::Error);
                            return;
                        };
                        de.bg_sel_data.start_pos = cur_pos;
                        de.bg_sel_data.end_pos = cur_pos; // Starts as empty square
                    }
                    if bg_interaction.dragged() {
                        let Some(cur_pos) = ui.ctx().pointer_interact_pos() else {
                            log_write("Failed to get pointer_interact_pos in BG .dragged", LogLevel::Error);
                            return;
                        };
                        de.bg_sel_data.end_pos = cur_pos;
                        let drag_rect: Rect = Rect::from_two_pos(de.bg_sel_data.start_pos, de.bg_sel_data.end_pos);
                        // Selection rectangle should look different if Control is held
                        if ui.input(|i| i.modifiers.ctrl) {
                            painter.rect_filled(drag_rect, 0.0, BG_SELECTION_FILL_INVERT);
                        } else {
                            painter.rect_filled(drag_rect, 0.0, BG_SELECTION_FILL);
                        }
                        painter.rect_stroke(drag_rect, 0.0, Stroke::new(1.0, BG_SELECTION_STROKE), egui::StrokeKind::Outside);
                        de.bg_sel_data.selecting_rect = drag_rect; // Pass the data on in
                    }
                    if bg_interaction.drag_stopped() {
                        log_write("Stopped dragging in draw_background", LogLevel::Debug);
                        let shift_held = ui.input(|i| i.modifiers.shift);
                        let ctrl_held = ui.input(|i| i.modifiers.ctrl);
                        if shift_held { // Add
                            de.bg_sel_data.selected_map_indexes.append(&mut temp_selected_indexes);
                            de.bg_sel_data.selected_map_indexes.sort();
                            de.bg_sel_data.selected_map_indexes.dedup();
                        } else if ctrl_held { // Remove
                            for removing_index in &temp_selected_indexes {
                                let found_pos = de.bg_sel_data.selected_map_indexes.iter().position(|&p| p == *removing_index);
                                if let Some(pos_found) = found_pos {
                                    de.bg_sel_data.selected_map_indexes.remove(pos_found);
                                }
                            }
                        } else { // Replace
                            de.bg_sel_data.selected_map_indexes = std::mem::take(&mut temp_selected_indexes);
                            de.bg_sel_data.selected_map_indexes.sort();
                            de.bg_sel_data.selected_map_indexes.dedup();
                        }
                        temp_selected_indexes.clear();
                        de.bg_sel_data.dragging = false;
                        de.bg_sel_data.selecting_rect = Rect::NOTHING;
                        de.bg_sel_data.selection_width = de.bg_sel_data.get_selection_width(info.layer_width);
                        de.bg_sel_data.selection_height = de.bg_sel_data.get_selection_height(info.layer_width);
                    }
                    ////////////////////////
                    // MOUSE SINGLE CLICK //
                    ////////////////////////
                    if bg_interaction.clicked() {
                        // Deselect
                        //log_write(format!("Clearing BG selection"), LogLevel::Debug);
                        de.bg_sel_data.clear();
                    }
                    if bg_interaction.secondary_clicked() {
                        // Place tile //
                        // Lots of opportunities to crash here, so include Debug
                        log_write("Stamping Brush to BG", LogLevel::Debug);
                        if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let local_pos = pointer_pos - true_grid_rect.min;
                            let mut base_tile_x: u32 = (local_pos.x/TILE_WIDTH_PX) as u32;
                            if base_tile_x % 2 != 0 { // Don't paste at odd positions
                                base_tile_x -= 1; // Move to even position
                            }
                            let mut base_tile_y: u32 = (local_pos.y/TILE_HEIGHT_PX) as u32;
                            if base_tile_y % 2 != 0 { // Don't paste at odd positions
                                base_tile_y -= 1; // Move to even position
                            }
                            let mut tile_index: u32 = 0;
                            for tile in &de.current_brush.tiles {
                                let offset_x = tile_index % (de.current_brush.width as u32);
                                let offset_y = tile_index / (de.current_brush.width as u32);
                                let true_x = base_tile_x + offset_x;
                                let true_y = base_tile_y + offset_y;
                                if true_y >= info.layer_height as u32 {
                                    tile_index += 1;
                                    continue;
                                }
                                if true_x >= info.layer_width as u32 {
                                    tile_index += 1;
                                    continue;
                                }
                                let map_index = true_y * (info.layer_width as u32) + true_x;
                                if *tile != 0x0000 { // Don't overwrite tiles with blanks
                                    de.loaded_map.place_bg_tile_at_map_index(info.which_bg, map_index, *tile);
                                }
                                tile_index += 1;
                            }
                            de.graphics_update_needed = true;
                            de.unsaved_changes = true;
                        } else {
                            log_write("Failed to get pointer when stamping Brush", LogLevel::Error);
                        }
                    }
                    if bg_interaction.middle_clicked() {
                        if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let local_pos = pointer_pos - true_grid_rect.min;
                            let tile_x: u32 = (local_pos.x/TILE_WIDTH_PX) as u32;
                            let tile_y: u32 = (local_pos.y/TILE_HEIGHT_PX) as u32;
                            let tile_index: u32 = tile_y * grid_width + tile_x;
                            println!("=== Mouse clicked at 0x{:X},0x{:X} on BG {} ===",tile_x, tile_y, whichbg);
                            println!("Map tile index: 0x{:X}",tile_index);
                            let clicked_map_tile = &map_tiles.tiles[tile_index as usize];
                            println!("{}",clicked_map_tile);
                            de.selected_preview_tile = Some(clicked_map_tile.tile_id as usize);
                            let mut adjusted_pal = clicked_map_tile.palette_id as i16 + layer._pal_offset as i16 + 1;
                            println!("16 Adjusted Palette: 0x{:X}",adjusted_pal);
                            adjusted_pal = adjusted_pal.clamp(0x0, 0xF);
                            // TODO: Scroll to it in the tiles window?
                            de.tile_preview_pal = adjusted_pal as usize;
                            de.needs_bg_tile_refresh = true;
                            // Now print the actual tile values
                            if !info.is_256_colorpal_mode() {
                                let array_start: usize = clicked_map_tile.tile_id as usize * 32;
                                let array_end: usize = array_start + 32;
                                let pixels = pixel_tiles[array_start..array_end].to_vec();
                                utils::print_vector_u8(&pixels);
                            } else {
                                // 256
                                let array_start: usize = clicked_map_tile.tile_id as usize * 64;
                                let array_end: usize = array_start + 64;
                                let pixels = pixel_tiles[array_start..array_end].to_vec();
                                utils::print_vector_u8(&pixels);
                            }
                            println!("=== End Click Debug ===");
                        }
                    }
                }
                // End Interactivity
            } else {
                // Not a guarantee that every background will have pixel tiles
                //log_write(format!("Pixel Tiles not found on background '{}' when drawing",&whichbg), LogLevel::Error);
            }
        } else {
            log_write(format!("Map Tiles not found on background '{}' when drawing",&whichbg), LogLevel::Error);
        }
        // Generic Red 2x2 Rectangle and Green Brush Preview
        if is_selected_layer {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - true_grid_rect.min;
                let mut tile_x: u32 = (local_pos.x/TILE_WIDTH_PX) as u32;
                let mut tile_y: u32 = (local_pos.y/TILE_HEIGHT_PX) as u32;
                de.tile_hover_pos.x = tile_x as f32;
                de.tile_hover_pos.y = tile_y as f32;
                // Ensure its position is even
                if tile_x % 2 != 0 {
                    tile_x -= 1;
                }
                if tile_y % 2 != 0 {
                    tile_y -= 1;
                }
                de.latest_square_pos_level_space = Pos2::new(tile_x as f32, tile_y as f32);
                if !de.current_brush.tiles.is_empty() {
                    let width = de.current_brush.width as f32;
                    let height = de.current_brush.height as f32;
                    let brush_rect = Rect::from_min_size(
                    true_grid_rect.min + Vec2::new((tile_x as f32) * TILE_WIDTH_PX, (tile_y as f32) * TILE_HEIGHT_PX),
                    Vec2 { x: TILE_WIDTH_PX * width, y: TILE_HEIGHT_PX * height });
                    ui.painter().rect_stroke(brush_rect, 0.0, Stroke::new(1.0, Color32::GREEN), egui::StrokeKind::Outside);
                }
                let square_rect = Rect::from_min_size(
                    true_grid_rect.min + Vec2::new((tile_x as f32) * TILE_WIDTH_PX, (tile_y as f32) * TILE_HEIGHT_PX),
                    Vec2 { x: TILE_WIDTH_PX * 2.0, y: TILE_HEIGHT_PX * 2.0 });
                ui.painter().rect_stroke(square_rect, 0.0, Stroke::new(1.0, Color32::RED), egui::StrokeKind::Outside);
            }
        }
    }
}

fn local_pos_to_col_index(local_pos: &Vec2, std_grid_width: u32) -> u32 {
    let tile_x: u32 = (local_pos.x/(TILE_WIDTH_PX*2.0)) as u32;
    let tile_y: u32 = (local_pos.y/(TILE_HEIGHT_PX*2.0)) as u32;
    let tile_index: u32 = tile_y * (std_grid_width/2) + tile_x;
    tile_index
}

fn draw_tile(
    tile: &MapTileRecordData,
    ctx: &Context, pixel_tiles: &[u8],
    painter: &Painter, tc: &mut TileCache,
    true_rect: &Rect, selected: bool,
    dim: bool,
    create_texture_image: impl Fn(&MapTileRecordData, &[u8]) -> ColorImage, texture_name: &str
) {
    puffin::profile_function!();
    if let Some(t) = get_cached_texture(tc,tile.palette_id as usize, tile.tile_id as usize) {
        let uvs = utils::get_uvs_from_tile(tile);
        let color = match (dim, selected) {
            (true, _) => Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0x40),
            (_, true) => Color32::PURPLE,
            _ => Color32::WHITE,
        };
        painter.image(t.id(), *true_rect, uvs, color);
    } else {
        let color_image = create_texture_image(tile, pixel_tiles);
        set_cached_texture(
            tc, tile.palette_id as usize, tile.tile_id as usize,
            ctx.load_texture(texture_name, color_image, egui::TextureOptions::NEAREST),
        );
    }
}

pub fn draw_tile_16(
    tile: &MapTileRecordData, palette: &Palette,
    ctx: &Context, pixel_tiles: &[u8],
    painter: &Painter, tc: &mut TileCache,
    true_rect: &Rect, selected: bool,
    dim: bool
) {
    puffin::profile_function!();
    draw_tile(tile, ctx, pixel_tiles, painter, tc, true_rect, selected, dim,
        |tile, pixel_tiles| {
            let byte_array = utils::get_pixel_bytes_16(pixel_tiles, &tile.tile_id);
            let nibble_array = utils::pixel_byte_array_to_nibbles(&byte_array);
            utils::color_image_from_pal(palette, &nibble_array)
        }, "tile16"
    );
}

pub fn draw_tile_256(
    tile: &MapTileRecordData, palette256: &Palette,
    ctx: &Context, pixel_tiles: &[u8],
    painter: &Painter, tc: &mut TileCache,
    true_rect: &Rect, selected: bool,
    dim: bool
) {
    puffin::profile_function!();
    draw_tile(tile, ctx, pixel_tiles, painter, tc, true_rect, selected, dim,
        |tile, pixel_tiles| {
            let byte_array = utils::get_pixel_bytes_256(pixel_tiles, &tile.tile_id);
            utils::color_image_from_pal(palette256, &byte_array)
        }, "tile256"
    );
}
