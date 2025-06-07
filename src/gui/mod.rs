use crate::data::sprites::LevelSprite;

#[allow(clippy::module_inception)]
pub mod gui;
pub mod toppanel;
pub mod sidepanel;
pub mod windows;
pub mod maingrid;
pub mod spritepanel;
pub mod spritesettings;

pub trait SpriteSettings {
    /// Generate a UI that modifies it
    fn show_ui(&mut self, ui: &mut egui::Ui) -> egui::Response;
    /// Create 4-padded settings vector
    fn compile(&self) -> Vec<u8>;
    /// Create it from the Sprite
    fn from_sprite(spr: &LevelSprite) -> Self;
}
