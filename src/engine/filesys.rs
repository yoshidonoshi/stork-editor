use std::{fmt::Display, path::PathBuf};

use ds_rom::rom::{raw, Rom, RomLoadOptions};
use crate::utils::{self, log_write, LogLevel};

/// Only a placeholder for now
pub struct RomExtractError {
    pub cause: String
}
impl RomExtractError {
    pub fn new(cause: &String) -> Self {
        Self {
            cause: cause.clone()
        }
    }
}
impl Display for RomExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error extracting ROM: '{}'", &self.cause)
    }
}

pub fn extract_rom_files(nds_file: &PathBuf, output_dir: &PathBuf) -> Result<PathBuf,RomExtractError> {
    let raw_rom = raw::Rom::from_file(nds_file);
    if raw_rom.is_err() {
        let open_fail = format!("Failed to open ROM file '{}'", &nds_file.display());
        log_write(open_fail.clone(), utils::LogLevel::ERROR);
        return Err(RomExtractError::new(&open_fail));
    }
    let raw_rom = raw_rom.unwrap();
    let rom = Rom::extract(&raw_rom);
    if rom.is_err() {
        let extract_err = format!("Failed to extract ROM contents");
        log_write(extract_err.clone(), utils::LogLevel::ERROR);
        return Err(RomExtractError::new(&extract_err));
    }
    let rom = rom.unwrap();
    let save_result = rom.save(&output_dir, None);
    if save_result.is_err() {
        let save_fail = format!("Failed to save extracted ROM contents");
        log_write(save_fail.clone(), utils::LogLevel::ERROR);
        Err(RomExtractError::new(&save_fail))
    } else {
        log_write(format!("ROM contents extracted to '{}' successfully", &output_dir.display()), utils::LogLevel::LOG);
        let ret_dir = output_dir.clone();
        Ok(ret_dir)
    }
}

// Only a placeholder for now
pub struct RomGenerateError{}

pub fn generate_rom(config: &String, new_nds_file: &String) -> Result<(),RomGenerateError> {
    log_write(format!("This will take a long time..."), utils::LogLevel::LOG);
    let rom = Rom::load(&config, RomLoadOptions::default());
    if rom.is_err() {
        utils::log_write(format!("Failed to load directory '{}'",&config), utils::LogLevel::ERROR);
        return Err(RomGenerateError{});
    } else {
        log_write(format!("Config processed successfully"), LogLevel::LOG);
    }
    let rom = rom.unwrap();
    let raw_rom = rom.build(None);
    if raw_rom.is_err() {
        utils::log_write("Failed to build ROM".to_string(), utils::LogLevel::ERROR);
        return Err(RomGenerateError{});
    }
    let raw_rom = raw_rom.unwrap();
    let save_result = raw_rom.save(new_nds_file);
    if save_result.is_err() {
        utils::log_write(format!("Failed to generate ROM '{}'",new_nds_file), utils::LogLevel::ERROR);
        Err(RomGenerateError{})
    } else {
        utils::log_write(format!("Generated ROM '{}' successfully",new_nds_file), utils::LogLevel::LOG);
        Ok(())
    }
}