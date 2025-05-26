use egui::{Align2, Color32, Context, FontId, Image, Painter, Pos2, Rect, Response, Stroke, TextureHandle, Vec2};
use uuid::Uuid;

use crate::{data::{area::{TriggerData, AREA_RECT_COLOR, AREA_RECT_COLOR_SELECTED}, backgrounddata::BackgroundData, scendata::colz::{draw_collision, COLLISION_BG_COLOR, COLLISION_OUTLINE_COLOR, COLLISION_SQUARE}, sprites::{draw_sprite, LevelSprite}, types::{get_cached_texture, set_cached_texture, CurrentLayer, MapTileRecordData, Palette, TileCache}}, engine::displayengine::DisplayEngine, utils::{color_image_from_pal, distance, get_pixel_bytes_16, get_pixel_bytes_256, get_sin_cos_table_value, get_uvs_from_tile, log_write, pixel_byte_array_to_nibbles, print_vector_u8, settings_to_string, LogLevel}};

const TILE_WIDTH_PX: f32 = 8.0;
const TILE_HEIGHT_PX: f32 = 8.0;
const TILE_RECT: Vec2 = Vec2::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
const TILE_OUTER_PADDING: f32 = 10.0;
const RECT_TRIM_PADDING_TILE: f32 = 1.0;
const SPRITE_RECT: Vec2 = Vec2::new(TILE_WIDTH_PX * 2.0, TILE_HEIGHT_PX * 2.0);
const SPRITE_BG_COLOR: Color32 = Color32::from_rgba_premultiplied(0xff, 0x00, 0xff, 0x40);
const SPRITE_BG_COLOR_SELECTED: Color32 = Color32::from_rgba_premultiplied(0x00, 0xff, 0x00, 0x40);
const FONT: FontId = FontId { size: 12.0, family: egui::FontFamily::Monospace };
const BG_SELECTION_FILL: Color32 = Color32::from_rgba_premultiplied(0x80, 0x65, 0xb5, 0xA0);
const BG_SELECTION_FILL_INVERT: Color32 = Color32::from_rgba_premultiplied(0x65, 0x80, 0xb5, 0xA0);
const BG_SELECTION_STROKE: Color32 = Color32::WHITE;

/// Active drawing for various visible data layers
/// 
/// Each one takes in the display data plus a UI reference, then combines the two
/// to create a drawn layer. This also includes logic to disable drawing the layer.
pub fn render_primary_grid(ui: &mut egui::Ui, de: &mut DisplayEngine, vrect: &Rect) {
    draw_background(ui, de, vrect, 3, de.display_settings.show_bg3);
    draw_background(ui, de, vrect, 2, de.display_settings.show_bg2);
    draw_background(ui, de, vrect, 1, de.display_settings.show_bg1);
    if de.display_settings.show_col {
        draw_collision_layer(ui, de, vrect);
    }
    if de.display_settings.show_breakable_rock {
        draw_breakable_rock(ui, de);
    }
    if de.display_settings.show_sprites {
        draw_sprites(ui, de, vrect);
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
    let bg_with_col = de.loaded_map.get_bg_with_colz();
    if bg_with_col.is_none() {
        return;
    }
    let bg_with_col = bg_with_col.unwrap();
    let bg_res = de.loaded_map.get_background(bg_with_col);
    if bg_res.is_none() {
        return;
    }
    let bg = bg_res.unwrap();
    let info_c = bg.get_info().unwrap().clone();
    let grid_width = info_c.layer_width as u32;
    let colz = bg.get_colz_mut();
    if colz.is_none() {
        return;
    }
    let col = colz.unwrap();
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
            let rect: Rect = Rect::from_min_size(top_left + Vec2::new(tile_x_px, tile_y_px), COLLISION_SQUARE);
            let col_bg_color = COLLISION_BG_COLOR;
            if *col_u8 == 0x1 { // Square, 95% of non-empty colliders (I checked)
                painter.rect_filled(rect, 0.0, col_bg_color);
                painter.rect_stroke(rect, 0.0, Stroke::new(1.0, COLLISION_OUTLINE_COLOR), egui::StrokeKind::Middle);
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
    if de.display_settings.current_layer == CurrentLayer::COLLISION {
        let col_sense_resp: Response = ui.interact(true_rect, egui::Id::new("col_tile_click"), egui::Sense::all());
        // Do it in three separate ones to avoid repeated input checking that won't be used
        if col_sense_resp.clicked() {
            // Add a new tile 
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - true_rect.min;
                let tile_index = local_pos_to_col_index(&local_pos, grid_width);
                if tile_index as usize >= col.col_tiles.len() {
                    log_write(format!("Index out of bounds: {} >= {}",tile_index,col.col_tiles.len()), LogLevel::ERROR);
                    return;
                }
                let _ = de.loaded_map.set_col_tile(bg_with_col, tile_index as u16, de.col_tile_to_place);
                de.graphics_update_needed = true;
                de.unsaved_changes = true;
            }
        } else if col_sense_resp.secondary_clicked() {
            // Clear the tile
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - true_rect.min;
                let tile_index = local_pos_to_col_index(&local_pos, grid_width);
                if tile_index as usize >= col.col_tiles.len() {
                    log_write(format!("Index out of bounds: {} >= {}",tile_index,col.col_tiles.len()), LogLevel::ERROR);
                    return;
                }
                // 0x00 is empty
                let _ = de.loaded_map.set_col_tile(bg_with_col, tile_index as u16, 0x00);
                de.graphics_update_needed = true;
                de.unsaved_changes = true;
            }
        } else if col_sense_resp.middle_clicked() {
            // Copy the tile (and show info)
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let local_pos = pointer_pos - true_rect.min;
                let tile_index = local_pos_to_col_index(&local_pos, grid_width);
                if tile_index as usize >= col.col_tiles.len() {
                    log_write(format!("Index out of bounds: {} >= {}",tile_index,col.col_tiles.len()), LogLevel::ERROR);
                    return;
                }
                let tile = &col.col_tiles[tile_index as usize];
                // Don't copy empty ones, that's for right clicking
                if *tile != 0x00 {
                    log_write(format!("Copied tile of type '0x{:X}' at index 0x{:X}",tile,tile_index), LogLevel::LOG);
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
            let cur_pos_res = ui.ctx().pointer_interact_pos();
            if cur_pos_res.is_none() {
                log_write("Failed to get pointer_interact_pos in col .dragged", LogLevel::ERROR);
                return;
            }
            de.col_selector_status.end_pos = cur_pos_res.unwrap();
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
    let top_left_screen: Pos2 = ui.min_rect().min;
    let area_res = de.loaded_map.get_area();
    if area_res.is_none() {
        return;
    }
    let area: &TriggerData = area_res.unwrap();
    for trigger in &area.triggers {
        let rect = trigger.get_rect(top_left_screen, TILE_WIDTH_PX, TILE_HEIGHT_PX);
        if de.trigger_settings.selected_uuid == trigger.uuid {
            ui.painter().rect_filled(rect, 0.0, AREA_RECT_COLOR_SELECTED);
        } else {
            ui.painter().rect_filled(rect, 0.0, AREA_RECT_COLOR);
        }
    }

    if de.display_settings.current_layer == CurrentLayer::TRIGGERS {
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
    let top_left_screen: Pos2 = ui.min_rect().min;
    // It's read-only, we aren't changing it
    let blkz_res = de.loaded_map.get_blkz().cloned();
    if blkz_res.is_none() {
        return;
    }
    let blkz_data = blkz_res.unwrap();

    let colz_layer = de.loaded_map.get_bg_with_colz();
    if colz_layer.is_none() {
        log_write("There is no layer with COLZ", LogLevel::ERROR);
        return;
    }
    let bg = de.loaded_map.get_background(colz_layer.unwrap()).expect("Already confirmed COLZ");
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
            log_write(format!("palette id for render too high in draw_breakable_rock: {}", render_pal_id), LogLevel::ERROR);
            continue;
        }
        let palette = &de.bg_palettes[render_pal_id];
        let pixel_tiles = bg.pixel_tiles_preview.clone().expect("There should be pixel tiles on the background with COLZ");
        draw_blkz_tile(tile, palette, &pixel_tiles, &true_rect,ui.ctx(),ui.painter());
        // Placement is good!
        //ui.painter().rect_filled(true_rect, 0.0, Color32::RED);
        tile_index += 1;
    }
}

fn draw_blkz_tile(
    tile: &MapTileRecordData, palette: &Palette,
    pixel_tiles: &Vec<u8>, true_rect: &Rect,
    ctx: &Context, painter: &Painter
) {
    let byte_array = &get_pixel_bytes_16(pixel_tiles, &tile.tile_id);
    let nibble_array = pixel_byte_array_to_nibbles(byte_array);
    let color_image = color_image_from_pal(palette, &nibble_array);
    let handle = ctx.load_texture("tile16", color_image, egui::TextureOptions::NEAREST);
    let uvs = get_uvs_from_tile(tile);
    painter.image(handle.id(), *true_rect, uvs, Color32::WHITE);
}

fn draw_entrances(ui: &mut egui::Ui, de: &mut DisplayEngine) {
    let top_left: Pos2 = ui.min_rect().min;
    if de.map_index.is_none() {
        // Just don't do anything
        return;
    }
    let map_index = de.map_index.unwrap();
    let maps_count = de.loaded_course.level_map_data.len();
    if map_index >= maps_count {
        // This will cause an overflow panic
        log_write(format!(
            "Map Index is somehow greater than level map data length ({} >= {})",
            &map_index,&maps_count), LogLevel::FATAL);
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
    let top_left: Pos2 = ui.min_rect().min;
    if de.map_index.is_none() {
        // Just don't do anything
        return;
    }
    let map_index = de.map_index.unwrap();
    let maps_count = de.loaded_course.level_map_data.len();
    if map_index >= maps_count {
        // This will cause an overflow panic if not stopped
        log_write(format!(
            "Map Index is somehow greater than level map data length ({} >= {})",
            &map_index,&maps_count), LogLevel::FATAL);
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
    let arm9 = de.loaded_arm9.clone().expect("ARM9 must exist");
    let top_left: Pos2 = ui.min_rect().min;
    if let Some(path_database) = &de.path_data {
        let mut _line_index: usize = 0;
        for line in &path_database.lines {
            let mut line_points: Vec<Pos2> = Vec::new();
            let mut _point_index: usize = 0;
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
                // let text_to_draw = format!("{}-{}",&line_index,&point_index);
                // ui.painter().text(
                //     rect.left_top(), Align2::LEFT_TOP,
                //     text_to_draw,
                //     FontId { size: 12.0, family: egui::FontFamily::Monospace },
                //     Color32::WHITE
                // );
                if point.distance >= 0 {
                    let test_val = get_sin_cos_table_value(&arm9, point.angle as u16);
                    //println!("test_val: {:?}", test_val);
                    let _end_pos: Pos2 = Pos2::new(true_pos.x + (test_val.x as f32), true_pos.y + (test_val.y as f32));
                    //ui.painter().line(vec![true_pos,end_pos], Stroke::new(3.0,  if point_selected {Color32::GREEN} else { Color32::PURPLE } ));
                }
                _point_index += 1;
            }
            _line_index += 1;
            ui.painter().line(line_points, Stroke::new(1.0,
                if path_selected { Color32::LIGHT_RED } else { Color32::RED }
            ));
        }
        // Interactivity
        if de.display_settings.current_layer == CurrentLayer::PATHS {
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
                            let distance = distance(point_pos, local_pos.to_pos2());
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
        }
    }
}

fn draw_sprites(ui: &mut egui::Ui, de: &mut DisplayEngine, vrect: &Rect) {
    let top_left: Pos2 = ui.min_rect().min;
    let mut update_map: bool = false;
    // If this always fires, it will block COLZ clicks
    let mut click_fallback_response: Option<Response> = Option::None;
    if de.display_settings.current_layer == CurrentLayer::SPRITES {
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
            drawn_rects.push(rect.clone());

            if de.selected_sprite_uuids.contains(&level_sprite.uuid) {
                ui.painter().rect_filled(rect, 0.0, SPRITE_BG_COLOR_SELECTED);
            } else {
                ui.painter().rect_filled(rect, 0.0, SPRITE_BG_COLOR);
            }
            ui.painter().text(
                true_pos, Align2::LEFT_TOP,
                format!("{:02X}",level_sprite.object_id),
                FONT.clone(), Color32::WHITE
            );
        }

        // Interactivity
        if de.display_settings.current_layer == CurrentLayer::SPRITES {
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
                        de.latest_sprite_settings = settings_to_string(&level_sprite.settings);
                    }
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
                        log_write("Started dragging sprite", LogLevel::DEBUG);
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
                            let og_sprite_data = de.get_loaded_sprite_by_uuid(selspr);
                            if og_sprite_data.is_none() {
                                log_write(format!("Sprite Uuid '{}' not found when moving",selspr), LogLevel::ERROR);
                                continue;
                            }
                            let og_sprite_data = og_sprite_data.unwrap();
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
    if de.display_settings.current_layer == CurrentLayer::SPRITES {
        if let Some(cfr) = &click_fallback_response {
            if cfr.clicked() { // Clicked on empty background
                de.selected_sprite_uuids.clear();
            }
            if cfr.secondary_clicked() { // Right clicked on empty background = place
                log_write("Placing new sprite from right click...", LogLevel::DEBUG);
                if de.selected_sprite_to_place.is_none() {
                    log_write("Could not place sprite, none selected to add", LogLevel::DEBUG);
                    return;
                }
                // Retrieve the base sprite ID to create, usually set by Add Sprite
                let new_sprite_id = de.selected_sprite_to_place.unwrap();
                if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                    let local_pos = pointer_pos - ui.min_rect().min;
                    let base_tile_x: u16 = (local_pos.x/TILE_WIDTH_PX) as u16;
                    let base_tile_y: u16 = (local_pos.y/TILE_HEIGHT_PX) as u16;
                    let new_uuid = de.loaded_map.add_new_sprite_at(new_sprite_id, base_tile_x, base_tile_y, &de.sprite_metadata_copy);
                    log_write(format!("Placed sprite with UUID {}",new_uuid.to_string()), LogLevel::DEBUG);
                    de.selected_sprite_uuids = vec![new_uuid]; // Select only it
                    de.unsaved_changes = true;
                    update_map = true;
                } else {
                    log_write("Could not get pointer pos when right clicking Sprite", LogLevel::ERROR);
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
    // This is used to offset the drawing to the window
    //   Otherwise it doesn't "stick to the window"
    let top_left: Pos2 = ui.min_rect().min;
    // These will be used for rendering fewer tiles to save CPU
    let leftmost_tile = vrect.left() / TILE_WIDTH_PX;
    let rightmost_tile = vrect.right() / TILE_WIDTH_PX;
    let uppermost_tile = vrect.top() / TILE_HEIGHT_PX;
    let bottommost_tile = vrect.bottom() / TILE_HEIGHT_PX;
    #[allow(unused_assignments)] // Unknown why this is needed
    let mut bg_layer_opt: &Option<BackgroundData> = &Option::None;
    #[allow(unused_assignments)] // Same here
    let mut tc: Option<&mut TileCache> = Option::None;
    match whichbg {
        1 => {
            bg_layer_opt = &de.bg_layer_1;
            tc = Some(&mut de.tile_cache_bg1);
        }
        2 => {
            bg_layer_opt = &de.bg_layer_2;
            tc = Some(&mut de.tile_cache_bg2);
        }
        3 => {
            bg_layer_opt = &de.bg_layer_3;
            tc = Some(&mut de.tile_cache_bg3);
        }
        _ => {
            log_write(format!("Unusual whichbg value in draw_background: '{}'",whichbg), LogLevel::ERROR);
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
        let true_rect = ui.min_rect();
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
                        log_write(format!("Palette ID was too high when attempting to draw tile on bg {} (was 0x{:X})",whichbg,pal_id), LogLevel::ERROR);
                        log_write(format!("Offending MapTile data: {}",map_tile), LogLevel::LOG);
                        // skip
                        map_index += 1;
                        continue;
                    }
                    let cur_pal = &de.bg_palettes[pal_id];
                    // Check if the tile id is out of bounds (Often caused by missing ANMZ data)
                    if map_tile.tile_id as usize >= pixel_tiles.len() {
                        log_write(format!("Tile ID was too high when attempting to draw tile on bg {} (was 0x{:X})",whichbg,map_tile.tile_id), LogLevel::ERROR);
                        log_write(format!("Offending MapTile data: {}",map_tile), LogLevel::LOG);
                        // skip
                        map_index += 1;
                        continue;
                    }
                    // This is the actual rectangle the tile will be rendered in
                    let true_rect: Rect = Rect::from_min_size(top_left + Vec2::new(tile_x_px, tile_y_px), TILE_RECT);
                    let mut selected: bool = false;
                    if de.bg_sel_data.selecting_rect.intersects(true_rect) && is_selected_layer {
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
                    let dim = (!is_selected_layer && is_cur_lay_bg) || de.display_settings.current_layer == CurrentLayer::COLLISION;
                    if let Some(tilecache) = &mut tc {
                        if !info.is_256_colorpal_mode() {
                            draw_tile_16(
                                map_tile, cur_pal, ctx, pixel_tiles,
                                painter, tilecache,
                                &true_rect, selected,dim);
                        } else {
                            if let Some(pltb) = layer.get_pltb() {
                                if pltb.palettes.is_empty() {
                                    log_write("PLTB palettes were empty when trying to draw 256 tile!".to_owned(), LogLevel::ERROR);
                                } else {
                                    draw_tile_256(
                                        map_tile, &pltb.palettes[0], ctx,
                                        pixel_tiles, painter, tilecache,
                                        &true_rect, selected, dim);
                                }
                            } else {
                                log_write(format!("Failed to find PLTB data for tile drawing on bg '{}'",info.which_bg), LogLevel::ERROR);
                            }
                        }
                        
                    }
                    map_index += 1;
                }
                // Interactivity //
                if is_selected_layer {
                    let interaction_id = egui::Id::new(format!("map_tile_interact_{}",whichbg));
                    // all() because it uses click, drag, and hover
                    let bg_interaction = ui.interact(true_rect, interaction_id, egui::Sense::all());
                    if bg_interaction.drag_started() {
                        log_write("Started dragging in BG render function", LogLevel::DEBUG);
                        de.bg_sel_data.dragging = true;
                        let cur_pos_res = ui.ctx().pointer_interact_pos();
                        if cur_pos_res.is_none() {
                            // This has failed before, somehow, so don't panic
                            log_write("Failed to get pointer_interact_pos in BG .drag_started", LogLevel::ERROR);
                            return;
                        }
                        let cur_pos: Pos2 = cur_pos_res.unwrap();
                        de.bg_sel_data.start_pos = cur_pos;
                        de.bg_sel_data.end_pos = cur_pos; // Starts as empty square
                    }
                    if bg_interaction.dragged() {
                        let cur_pos_res = ui.ctx().pointer_interact_pos();
                        if cur_pos_res.is_none() {
                            log_write("Failed to get pointer_interact_pos in BG .dragged", LogLevel::ERROR);
                            return;
                        }
                        de.bg_sel_data.end_pos = cur_pos_res.unwrap();
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
                        log_write("Stopped dragging in draw_background", LogLevel::DEBUG);
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
                            de.bg_sel_data.selected_map_indexes = temp_selected_indexes.clone();
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
                        //log_write(format!("Clearing BG selection"), LogLevel::DEBUG);
                        de.bg_sel_data.clear();
                    }
                    if bg_interaction.secondary_clicked() {
                        // Place tile //
                        // Lots of opportunities to crash here, so include Debug
                        log_write("Stamping Brush to BG", LogLevel::DEBUG);
                        if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let local_pos = pointer_pos - true_rect.min;
                            let mut base_tile_x: u32 = (local_pos.x/TILE_WIDTH_PX) as u32;
                            if base_tile_x % 2 != 0 { // Don't paste at odd positions
                                base_tile_x -= 1; // Move to even position
                            }
                            let mut base_tile_y: u32 = (local_pos.y/TILE_HEIGHT_PX) as u32;
                            if base_tile_y % 2 != 0 { // Don't paste at odd positions
                                base_tile_y -= 1; // Move to even position
                            }
                            let mut tile_index = 0;
                            for tile in &de.current_brush.tiles {
                                let offset_x = tile_index % de.current_brush.width;
                                let offset_y = tile_index / de.current_brush.width;
                                let true_x = base_tile_x + offset_x as u32;
                                let true_y = base_tile_y + offset_y as u32;
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
                                    de.loaded_map.place_bg_tile_at_map_index(info.which_bg, map_index, tile);
                                }
                                tile_index += 1;
                            }
                            de.graphics_update_needed = true;
                            de.unsaved_changes = true;
                        } else {
                            log_write("Failed to get pointer when stamping Brush", LogLevel::ERROR);
                        }
                    }
                    if bg_interaction.middle_clicked() {
                        // DEBUG, maybe remove or limit eventually?
                        if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let local_pos = pointer_pos - true_rect.min;
                            let tile_x: u32 = (local_pos.x/TILE_WIDTH_PX) as u32;
                            let tile_y: u32 = (local_pos.y/TILE_HEIGHT_PX) as u32;
                            let tile_index: u32 = tile_y * grid_width + tile_x;
                            println!("=== Mouse clicked at 0x{},0x{} on BG {} ===",tile_x, tile_y, whichbg);
                            println!("Map tile index: 0x{:X}",tile_index);
                            let clicked_map_tile = &map_tiles.tiles[tile_index as usize];
                            println!("{}",clicked_map_tile);
                            // Now print the actual tile values
                            if !info.is_256_colorpal_mode() {
                                let array_start: usize = clicked_map_tile.tile_id as usize * 32;
                                let array_end: usize = array_start + 32;
                                let pixels = pixel_tiles[array_start..array_end].to_vec();
                                print_vector_u8(&pixels);
                            } else {
                                // 256
                                let array_start: usize = clicked_map_tile.tile_id as usize * 64;
                                let array_end: usize = array_start + 64;
                                let pixels = pixel_tiles[array_start..array_end].to_vec();
                                print_vector_u8(&pixels);
                            }
                            println!("=== End Click Debug ===");
                        }
                    }
                }
                // End Interactivity
            } else {
                // Not a guarantee that every background will have pixel tiles
                //log_write(format!("Pixel Tiles not found on background '{}' when drawing",&whichbg), LogLevel::ERROR);
            }
        } else {
            log_write(format!("Map Tiles not found on background '{}' when drawing",&whichbg), LogLevel::ERROR);
        }
        // Generic 2x2 Rectangle
        if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
            let local_pos = pointer_pos - true_rect.min;
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
            //println!("x/y: 0x{:X}/0x{:X}",tile_x,tile_y);
            let square_rect = Rect::from_min_size(
                top_left + Vec2::new((tile_x as f32) * TILE_WIDTH_PX, (tile_y as f32) * TILE_HEIGHT_PX),
                Vec2 { x: TILE_WIDTH_PX * 2.0, y: TILE_HEIGHT_PX * 2.0 });
            ui.painter().rect_stroke(square_rect, 0.0, Stroke::new(1.0, Color32::RED), egui::StrokeKind::Outside);
        }
    }
}

fn local_pos_to_col_index(local_pos: &Vec2, std_grid_width: u32) -> u32 {
    let tile_x: u32 = (local_pos.x/(TILE_WIDTH_PX*2.0)) as u32;
    let tile_y: u32 = (local_pos.y/(TILE_HEIGHT_PX*2.0)) as u32;
    let tile_index: u32 = tile_y * (std_grid_width/2) + tile_x;
    tile_index
}

pub fn draw_tile_16(
    tile: &MapTileRecordData, palette: &Palette,
    ctx: &Context, pixel_tiles: &Vec<u8>,
    painter: &Painter, tc: &mut TileCache,
    true_rect: &Rect, selected: bool,
    dim: bool
) {
    // See if the texture already exists in the cache, will save much processing power
    let tex_handle_opt: &Option<egui::TextureHandle> = get_cached_texture(tc,tile.palette_id as usize, tile.tile_id as usize);
    let mut tex_handle_opt_2: Option<TextureHandle> = Option::None;
    if tex_handle_opt.is_none() {
        let byte_array = &get_pixel_bytes_16(pixel_tiles, &tile.tile_id);
        let nibble_array = pixel_byte_array_to_nibbles(byte_array);
        let color_image = color_image_from_pal(palette, &nibble_array);
        tex_handle_opt_2 = Some(ctx.load_texture("tile16", color_image, egui::TextureOptions::NEAREST));
    }

    let uvs = get_uvs_from_tile(tile);
    if let Some(t) = tex_handle_opt {
        if dim {
            painter.image(t.id(), *true_rect, uvs, Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0x40));
        } else if selected {
            painter.image(t.id(), *true_rect, uvs, Color32::PURPLE);
        } else {
            painter.image(t.id(), *true_rect, uvs, Color32::WHITE);
        }
    } else {
        set_cached_texture(tc, tile.palette_id as usize, tile.tile_id as usize, tex_handle_opt_2.unwrap());
        // It'll render it next time around
    }
    
}

pub fn draw_tile_256(
    tile: &MapTileRecordData, palette256: &Palette,
    ctx: &Context, pixel_tiles: &Vec<u8>,
    painter: &Painter, tc: &mut TileCache,
    true_rect: &Rect, selected: bool,
    dim: bool
) {
    let tex_handle_opt: &Option<egui::TextureHandle> = get_cached_texture(tc,tile.palette_id as usize, tile.tile_id as usize);
    let mut tex_handle_opt_2: Option<TextureHandle> = Option::None;
    if tex_handle_opt.is_none() {
        // Create the texture itself
        let byte_array = get_pixel_bytes_256(pixel_tiles, &tile.tile_id);
        let color_image = color_image_from_pal(palette256, &byte_array);
        tex_handle_opt_2 = Some(ctx.load_texture("tile256", color_image, egui::TextureOptions::NEAREST));
    }

    let uvs = get_uvs_from_tile(tile);
    if let Some(t) = tex_handle_opt {
        if dim {
            painter.image(t.id(), *true_rect, uvs, Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0x40));
        } else if selected {
            painter.image(t.id(), *true_rect, uvs, Color32::PURPLE);
        } else {
            painter.image(t.id(), *true_rect, uvs, Color32::WHITE);
        }
    } else {
        set_cached_texture(tc, tile.palette_id as usize, tile.tile_id as usize, tex_handle_opt_2.unwrap());
        // It'll render it next time around
    }
}
