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
    fn show_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.label("Behavior");
        egui::ComboBox::from_label("")
            .selected_text(match self.behavior {
                0 => "Wander",
                2 => "Chase",
                _ => "Unknown"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.behavior, 0, "Wander");
                ui.selectable_value(&mut self.behavior, 1, "Unknown");
                ui.selectable_value(&mut self.behavior, 2, "Chase");
            }            
        ).response
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
    fn show_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
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

pub struct RedArrowSign {
    pub kind: u8,
    pub order: i8
}
impl SpriteSettings for RedArrowSign {
    fn show_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.label("Kind");
        egui::ComboBox::new(egui::Id::new("kind"), "")
            .selected_text(match self.kind {
                0x0 => "Left Signpost".to_string(),
                0x1 => "Right Signpost".to_string(),
                0x2 => "Up Decal".to_string(),
                0x3 => "Up Right Decal".to_string(),
                0x4 => "Right Decal".to_string(),
                0x5 => "Down Right Decal".to_string(),
                0x6 => "Down Decal".to_string(),
                0x7 => "Down Left Decal".to_string(),
                0x8 => "Left Decal".to_string(),
                0x9 => "Up Left Decal".to_string(),
                _ => format!("Unknown: 0x{:X}",self.kind)
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.kind, 0, "Left Signpost");
                ui.selectable_value(&mut self.kind, 1, "Right Signpost");
                ui.selectable_value(&mut self.kind, 2, "Up Decal");
                ui.selectable_value(&mut self.kind, 3, "Up Right Decal");
                ui.selectable_value(&mut self.kind, 4, "Right Decal");
                ui.selectable_value(&mut self.kind, 5, "Down Right Decal");
                ui.selectable_value(&mut self.kind, 6, "Down Decal");
                ui.selectable_value(&mut self.kind, 7, "Down Left Decal");
                ui.selectable_value(&mut self.kind, 8, "Left Decal");
                ui.selectable_value(&mut self.kind, 9, "Up Left Decal");
            }            
        );
        ui.label("Order (WIP)");
        egui::ComboBox::new(egui::Id::new("order"), "")
            .selected_text(match self.order {
                -2 => "Before Yoshi".to_string(),
                -1 => "Behind Yoshi".to_string(),
                _ => format!("Unknown value: 0x{:X}",self.order)
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.order, -2, "Before Yoshi");
                ui.selectable_value(&mut self.order, -1, "Behind Yoshi");
            }            
        ).response
    }

    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u8(self.kind);
        let _ = comp.write_i8(self.order);
        let _padding = comp.write_u16::<LittleEndian>(0x0000);
        comp
    }

    fn from_sprite(spr: &LevelSprite) -> Self {
        Self {
            kind: spr.settings[0],
            order: spr.settings[1] as i8,
        }
    }
}
