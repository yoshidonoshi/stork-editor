use std::{fmt, io::{Cursor, Read, Write}};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use egui::{emath, pos2, Color32, ColorImage, Pos2, Rect, TextureHandle, Vec2};
use uuid::Uuid;

use crate::{engine::{compression::segment_wrap, displayengine::DisplayEngine}, utils::{self, color_image_from_pal, log_write, pixel_byte_array_to_nibbles, LogLevel}};

use super::{segments::DataSegment, types::Palette, TopLevelSegment};

/// Info on sprites to draw on the map, does not contain render data
#[derive(Clone,Debug,PartialEq)]
pub struct LevelSprite {
    pub object_id: u16,
    pub settings_length: u16,
    pub x_position: u16,
    pub y_position: u16,
    pub settings: Vec<u8>,
    pub uuid: Uuid
}
impl Default for LevelSprite {
    fn default() -> Self {
        Self {
            object_id: 0xfff0,
            settings_length: 0xfff0,
            x_position: 0x10, y_position: 0x10,
            settings: Vec::new(),
            uuid: Uuid::nil()
        }
    }
}
impl fmt::Display for LevelSprite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"LevelSprite [ id=0x{:X}, uuid={}, x_pos=0x{:X}, y_pos=0x{:X}, settings=[{}] ]",
            self.object_id, self.uuid, self.x_position, self.y_position, utils::settings_to_string(&self.settings))
    }
}
impl LevelSprite {
    pub fn from_cursor<T: ReadBytesExt>(rdr: &mut T)  -> Self {
        let mut spr = LevelSprite::default();
        spr.object_id = rdr.read_u16::<LittleEndian>().unwrap();
        spr.settings_length = rdr.read_u16::<LittleEndian>().unwrap();
        spr.x_position = rdr.read_u16::<LittleEndian>().unwrap();
        spr.y_position = rdr.read_u16::<LittleEndian>().unwrap();
        spr.uuid = Uuid::new_v4();
        let mut setting_index: u16 = 0;
        while setting_index < spr.settings_length {
            let setting_byte = rdr.read_u8().unwrap();
            spr.settings.push(setting_byte);
            setting_index += 1;
        }
        spr
    }
    #[allow(dead_code)] // only for debug, so may not be used
    pub fn from_vec(vec: &mut Vec<u8>) -> Self {
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(vec);
        LevelSprite::from_cursor(&mut rdr)
    }
    pub fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        // Maybe get rid of the warning for no applications someday
        let _ = comp.write_u16::<LittleEndian>(self.object_id);
        let _ = comp.write_u16::<LittleEndian>(self.settings_length);
        let _ = comp.write_u16::<LittleEndian>(self.x_position);
        let _ = comp.write_u16::<LittleEndian>(self.y_position);
        let _ = comp.write(self.settings.as_slice());
        comp
    }
    pub fn new(id: u16, x_pos: u16, y_pos: u16, settings: Vec<u8>) -> Self {
        LevelSprite {
            object_id: id, settings_length: settings.len() as u16,
            x_position: x_pos, y_position: y_pos,
            settings: settings.clone(), uuid: Uuid::new_v4()
        }
    }
}

#[derive(Clone,PartialEq,Debug,Default)]
pub struct LevelSpriteSet {
    pub sprites: Vec<LevelSprite>
}
impl LevelSpriteSet {
    pub fn new(byte_data: &[u8]) -> Self {
        let mut rdr = Cursor::new(byte_data);
        let seg_end: usize = byte_data.len();
        let mut seg: LevelSpriteSet = LevelSpriteSet::default();
        // If all goes well, the terminating position should be equal to the length
        while (rdr.position() as usize) != seg_end {
            if (rdr.position() as usize) > seg_end {
                log_write("Overflow when reading SETD", LogLevel::Error);
                break;
            }
            let sprite: LevelSprite = LevelSprite::from_cursor(&mut rdr);
            seg.sprites.push(sprite);
        }
        seg
    }

    pub fn trim(&mut self, width: u16, height: u16) -> usize {
        let mut deleted_count: usize = 0;
        let mut uuids_to_delete: Vec<Uuid> = Vec::new();
        for spr in &self.sprites {
            if spr.x_position >= width || spr.y_position >= height {
                uuids_to_delete.push(spr.uuid);
            }
        }
        for uid in &uuids_to_delete {
            let del_res = self.delete_sprite(*uid);
            if del_res.is_ok() {
                deleted_count += 1
            }
        }
        deleted_count
    }

    pub fn delete_sprite(&mut self, sprite_uuid: Uuid) -> Result<(),()> {
        let Some(pos) = self.sprites.iter().position(|x|x.uuid == sprite_uuid) else {
            return Err(());
        };
        self.sprites.remove(pos);
        Ok(())
    }
}
impl TopLevelSegment for LevelSpriteSet {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        for spr in &self.sprites {
            let mut sprite_bytes = spr.compile();
            comp.append(&mut sprite_bytes);
        }
        comp
    }
    
    fn wrap(&self) -> Vec<u8> {
        let comp_bytes: Vec<u8> = self.compile();
        segment_wrap(comp_bytes, "SETD".to_owned())
    }

    fn header(&self) -> String {
        String::from("SETD")
    }
}

#[derive(Debug,Clone)]
pub struct SpriteMetadata {
    pub sprite_id: u16,
    pub name: String,
    pub description: String,
    pub default_settings_len: u16
}
impl Default for SpriteMetadata {
    fn default() -> Self {
        Self {
            sprite_id: 0xfffe,
            name: "ERROR".to_owned(),
            description: "Error".to_owned(),
            default_settings_len: 0xfffe
        }
    }
}
impl fmt::Display for SpriteMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"SpriteMetadata [ sprite_id=0x{:X}, name='{}', description='{}', settings_len=0x{:X} ]",
            self.sprite_id,self.name,self.description,self.default_settings_len)
    }
}

fn get_graphics_segment(de: &mut DisplayEngine, archive_name_local_ext: String, segment_index: usize) -> SpriteGraphicsSegment {
    let arch_graphics = de.get_render_archive(&archive_name_local_ext);
    let graphics_segment = &arch_graphics.segments[segment_index];
    SpriteGraphicsSegment::from_data_segment(graphics_segment)
}

fn get_palette_from_segment(
    de: &mut DisplayEngine,
    archive_name_local_ext: String,
    segment_index: usize,
    pal_index: u32, pal_len: usize
) -> Palette {
    let arch_palette = de.get_render_archive(&archive_name_local_ext);
    let palette_segment = &arch_palette.segments[segment_index];
    Palette::from_segment_index(palette_segment, pal_index, pal_len)
}

pub fn draw_sprite(
    ui: &mut egui::Ui,
    rect: &Rect, sprite: &LevelSprite,
    de: &mut DisplayEngine,
    tile_dim: f32, selected: bool
) -> Vec<Rect> {
    match sprite.object_id {
        0x00 => { // Yellow Coin
            let gra = get_graphics_segment(de, "objset.arcz".to_owned(), 0);
            let pal = get_palette_from_segment(de, "objset.arcz".to_owned(), 0x7e, 0, 16);
            gra.render_sprite_frame(ui,0,&pal,&rect.left_top(),tile_dim,selected)
        }
        0x23 => {
            const PIPE_PALETTE: usize = 0x89;
            let direction: u16 = sprite.settings[0] as u16 + ((sprite.settings[1] as u16) << 8);
            let length: u16 = sprite.settings[2] as u16 + ((sprite.settings[3] as u16) << 8);
            // 0 and 1 is up and down, 2 and 3 is left and right
            let tileset_offset: usize = if direction < 2 { 0x13 } else { 0x12 }; // 02042e80, ~02042e9c
            let gra = get_graphics_segment(de, "objset.arcz".to_owned(), tileset_offset);
            let pal = get_palette_from_segment(de, "objset.arcz".to_owned(), PIPE_PALETTE, 0, 16);
            match direction {
                0x00 => { // Going down
                    let mut rects = vec![];
                    let mut top = gra.render_sprite_frame(ui,0,&pal,&rect.left_top(),tile_dim,selected);
                    for i in 0..length {
                        let new_rect = rect.left_top() + Vec2::new(0.0, (i as f32 * 16.0) + 16.0);
                        let mut mid = gra.render_sprite_frame(ui,1,&pal,&new_rect,tile_dim,selected);
                        rects.append(&mut mid);
                    }
                    let end_rect = rect.left_top() + Vec2::new(0.0, (length as f32 * 16.0) + 16.0);
                    let mut end = gra.render_sprite_frame(ui,2,&pal,&end_rect,tile_dim,selected);
                    rects.append(&mut top);
                    rects.append(&mut end);
                    rects
                }
                0x01 => { // Going up
                    let mut rects = vec![];
                    let start_rect = rect.left_top() + Vec2::new(0.0, -16.0);
                    let mut start = gra.render_sprite_frame(ui,3,&pal,&start_rect,tile_dim,selected);
                    rects.append(&mut start);
                    for i in 0..length {
                        let new_rect = rect.left_top() + Vec2::new(0.0, -16.0 - (i as f32 * 16.0) - 16.0);
                        let mut mid = gra.render_sprite_frame(ui,4,&pal,&new_rect,tile_dim,selected);
                        rects.append(&mut mid);
                    }
                    let end_rect = rect.left_top() + Vec2::new(0.0, -16.0 - (length as f32 * 16.0) - 16.0);
                    let mut end = gra.render_sprite_frame(ui,5,&pal,&end_rect,tile_dim,selected);
                    rects.append(&mut end);
                    rects
                }
                _ => {
                    // You'll need to add handlers for the others too
                    vec![]
                }
            }
        }
        0x28 => { // Flower Collectible
            let gra = get_graphics_segment(de, "objset.arcz".to_owned(), 0x16);
            let pal = get_palette_from_segment(de, "objset.arcz".to_owned(), 0x9b, 0, 16);
            gra.render_sprite_frame(ui,0,&pal,&rect.left_top(),tile_dim,selected)
        }
        0x3b => { // Red Coin
            let gra = get_graphics_segment(de, "objset.arcz".to_owned(), 0);
            let pal = get_palette_from_segment(de, "objset.arcz".to_owned(), 0x7e, 0, 16);
            gra.render_sprite_frame(ui,6,&pal,&rect.left_top(),tile_dim,selected)
        }
        0x9F => { // Hint Block
            let gra = get_graphics_segment(de, "objset.arcz".to_owned(), 0x5d);
            let pal = get_palette_from_segment(de, "objset.arcz".to_owned(), 0xa9, 0, 16);
            gra.render_sprite_frame(ui,0,&pal,&rect.left_top(),tile_dim,selected)
        }
        _ => vec![]
    }
}

#[derive(Debug,Clone)]
pub struct SpriteAnimFrame {
    build_offset: u16,
    hold_time: u8,
    frame_jump: i8,
    _pos: u64
}
impl Default for SpriteAnimFrame {
    fn default() -> Self {
        Self {
            build_offset: 0xffff, hold_time: 0xff,
            frame_jump: 0x00, _pos: 0xffffff
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpriteBuildData {
    pub tile_offset: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    pub flags: u16
}

#[derive(Debug,Clone,Default)]
pub struct SpriteGraphicsSegment {
    pub sprite_frames: Vec<SpriteAnimFrame>,
    internal_data: Vec<u8>
}
impl SpriteGraphicsSegment {
    pub fn from_data_segment(segment: &DataSegment) -> Self {
        //println!("from_data_segment");
        // Read Frames
        let mut ret: SpriteGraphicsSegment = SpriteGraphicsSegment::default();
        ret.internal_data = segment.internal_data.clone();
        //print_vector_u8(&ret.internal_data);
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(&segment.internal_data);
        let mut overflow_index: usize = 0;
        const OVERFLOW: usize = 0xfff0;
        while overflow_index < OVERFLOW {
            overflow_index += 1;
            let pos = rdr.position();
            let mut spr_frame: SpriteAnimFrame = SpriteAnimFrame::default();
            let offset = rdr.read_u16::<LittleEndian>().expect("u16 offset pulled");
            spr_frame.build_offset = offset;
            spr_frame.hold_time = rdr.read_u8().expect("u8 frame hold time");
            spr_frame.frame_jump = rdr.read_i8().expect("i8 frame jump");
            spr_frame._pos = pos;
            if offset == 0x0000 { // Impossible, must be end
                // Don't let it add a bad frame, but do reads to advance cursor anyway
                break;
            }
            ret.sprite_frames.push(spr_frame); // Move it on in
        }
        //log_write(format!("Loaded 0x{:X}/{} SpriteFrames",&ret.sprite_frames.len(),&ret.sprite_frames.len()), LogLevel::Log);
        // RDR is now at the start of BuildFrames! //
        ret
    }

    pub fn render_sprite_frame(&self,
        ui: &mut egui::Ui, frame_index: usize,
        pal: &Palette, top_left: &Pos2, tile_dim: f32,
        selected: bool
    ) -> Vec<Rect> {
        let sprite_frame = &self.sprite_frames[frame_index];

        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(&self.internal_data);
        rdr.set_position(sprite_frame.build_offset as u64 + sprite_frame._pos);
        let tile_offset = match rdr.read_u16::<LittleEndian>() {
            Err(error) => {
                log_write(format!("Failed to read tile_offset in render_sprite_frame: '{error}'"), LogLevel::Error);
                return Vec::new();
            }
            Ok(o) => o,
        };
        let x_offset: i16 = rdr.read_i16::<LittleEndian>().expect("render_sprite_frame: x_offset i16");
        let y_offset: i16 = rdr.read_i16::<LittleEndian>().expect("render_sprite_frame: y_offset i16");
        let flags: u16 = rdr.read_u16::<LittleEndian>().expect("render_sprite_frame: flags u16");
        // Get UVs //
        // Prior Stork implementation: (flags & 0b0010'0000'0000'0000) != 0
        let should_flip_v = (flags & 0b0010000000000000) != 0;
        // Prior Stork implementation: shouldFlipH = (flags & 0b0001'0000'0000'0000) != 0;
        let should_flip_h = (flags & 0b0001000000000000) != 0;
        // UV calculation
        let mut uvs = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        if should_flip_h && !should_flip_v {
            uvs = Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0));
        } else if !should_flip_h && should_flip_v {
            uvs = Rect::from_min_max(pos2(0.0, 1.0), pos2(1.0, 0.0));
        } else if should_flip_h && should_flip_v{
            uvs = Rect::from_min_max(pos2(1.0, 1.0), pos2(0.0, 0.0));
        }

        let bframe: SpriteBuildData = SpriteBuildData { tile_offset, x_offset, y_offset, flags };
        let pixels_start_position = bframe.tile_offset << 4;
        rdr.set_position(pixels_start_position as u64);
        let dims = get_sprite_dims_from_flag_value(bframe.flags & 0b11111);
        let tiles_count: u32 = (dims.x * dims.y) as u32;
        let mut rect_vec: Vec<Rect> = Vec::new();
        // We must get 32 bytes to get 64 tiles
        for n in 0..tiles_count { // In this example, 4 tiles are drawn because 2*2
            let mut buffer: Vec<u8> = vec![0;32];
            let _ = rdr.read_exact(&mut buffer);
            let nibbles_64: Vec<u8> = pixel_byte_array_to_nibbles(&buffer);
            let color_image: ColorImage = color_image_from_pal(pal, &nibbles_64);
            let tex: TextureHandle = ui.ctx().load_texture("sprite_tex", color_image, egui::TextureOptions::NEAREST);
            // Generate Rect from top_left
            let mut position: Pos2 = *top_left;
            // First do the per-position ones
            position.x += bframe.x_offset as f32;
            position.y += bframe.y_offset as f32;
            // Then do the tile offset ones
            let mut index_offset_x: f32 = (n as f32) % dims.x;
            if should_flip_h {
                // Untested so far
                index_offset_x = dims.x - 1.0 - index_offset_x;
            }
            let mut index_offset_y: f32 = ((n as f32) / dims.x).floor();
            if should_flip_v {
                // Works with width 4 height 2 flipped
                // let min_y = 0.0; // Always
                // let max_y = dims.y - 1.0;
                // index_offset_y = max_y - (index_offset_y - min_y);
                index_offset_y = dims.y - 1.0 - index_offset_y;
            }
            //println!("Index: x={},y={}",index_offset_x,index_offset_y);
            position.x += index_offset_x * tile_dim;
            position.y += index_offset_y * tile_dim;
            let rect = Rect::from_min_size(position, emath::Vec2::new(tile_dim,tile_dim));
            let mut tint: Color32 = Color32::WHITE;
            if selected {
                tint = Color32::GRAY;
            }
            ui.painter().image(tex.id(), rect, uvs, tint);
            rect_vec.push(rect);
        }
        rect_vec
    }

}

fn get_sprite_dims_from_flag_value(val: u16) -> Pos2 {
    match val {
        0x0 => Pos2::new(1.0,1.0),
        0x1 => Pos2::new(2.0,2.0),
        0x2 => Pos2::new(4.0,4.0),
        0x3 => Pos2::new(8.0,8.0),
        0x4 => Pos2::new(1.0,1.0),
        0x5 => Pos2::new(2.0,2.0),
        0x6 => Pos2::new(4.0,4.0),
        0x7 => Pos2::new(8.0,8.0),
        0x8 => Pos2::new(2.0,1.0),
        0x9 => Pos2::new(4.0,1.0),
        0xA => Pos2::new(4.0,2.0),
        0xB => Pos2::new(8.0,4.0),
        0xC => Pos2::new(2.0,1.0),
        0xD => Pos2::new(4.0,1.0),
        0xE => Pos2::new(4.0,2.0),
        0xF => Pos2::new(8.0,4.0),
        0x10 => Pos2::new(1.0,2.0),
        0x11 => Pos2::new(1.0,4.0),
        0x12 => Pos2::new(2.0,4.0),
        0x13 => Pos2::new(4.0,8.0),
        0x14 => Pos2::new(1.0,2.0),
        _ => {
            log_write(format!("Unknown Sprite Dim value: '{}'",val), LogLevel::Warn);
            Pos2::new(2.0, 2.0)
        }
    }
}
