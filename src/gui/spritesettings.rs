use byteorder::{LittleEndian, WriteBytesExt};

use crate::data::sprites::LevelSprite;

use super::SpriteSettings;

// pub struct MovingPlatform {
//     pub appearance: u8,
//     pub path_index: u8,
//     pub behavior: u8,
//     pub loop_to_start: bool,
//     pub direction_offset: i8,
//     pub fall_off: bool,
//     pub unknown1: i16,
//     pub speed: u32,
//     pub unknown2: i8,
//     pub unknown3: u32 // 3 bytes though
// }

pub struct ShyGuy {
    pub behavior: u8
}
impl SpriteSettings for ShyGuy {
    fn get_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            let drag_val = egui::DragValue::new(&mut self.behavior)
                .hexadecimal(2, false, true)
                .range(0..=2);
            ui.add(drag_val);
            ui.label("Behavior");
        }).response
    }

    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u32::<LittleEndian>(self.behavior as u32);
        comp
    }
    
    fn from_sprite(spr: &LevelSprite) -> Self {
        Self { behavior: spr.settings[0] }
    }
}

pub struct HintBlock {
    pub message: u16
}
impl SpriteSettings for HintBlock {
    fn get_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            let drag_val = egui::DragValue::new(&mut self.message)
                .hexadecimal(2, false, true)
                .range(0..=0x150);
            ui.add(drag_val);
            ui.label("Message ID");
        }).response
    }

    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u32::<LittleEndian>(self.message as u32);
        comp
    }

    fn from_sprite(spr: &LevelSprite) -> Self {
        let first_byte = spr.settings[0] as u16;
        let second_byte = spr.settings[1] as u16;
        Self { message: first_byte + (second_byte << 8) }
    }
}
