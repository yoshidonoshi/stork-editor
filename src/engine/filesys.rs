use std::{fmt::Display, path::PathBuf};

use ds_rom::rom::{raw, Rom, RomLoadOptions};
use crate::utils::{self, log_write, LogLevel};

/// Only a placeholder for now
pub struct RomExtractError {
    pub cause: String
}
impl RomExtractError {
    pub fn new(cause: &str) -> Self {
        Self {
            cause: cause.to_string()
        }
    }
}
impl Display for RomExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error extracting ROM: '{}'", &self.cause)
    }
}

pub fn extract_rom_files(nds_file: &PathBuf, output_dir: &PathBuf) -> Result<PathBuf,RomExtractError> {
    let Ok(raw_rom) = raw::Rom::from_file(nds_file) else {
        let open_fail = format!("Failed to open ROM file '{}'", &nds_file.display());
        log_write(open_fail.clone(), utils::LogLevel::Error);
        return Err(RomExtractError::new(&open_fail));
    };
    let Ok(rom) = Rom::extract(&raw_rom) else {
        let extract_err = "Failed to extract ROM contents".to_string();
        log_write(extract_err.clone(), utils::LogLevel::Error);
        return Err(RomExtractError::new(&extract_err));
    };
    match rom.save(&output_dir, None) {
        Ok(_) => {
            log_write(format!("ROM contents extracted to '{}' successfully", &output_dir.display()), utils::LogLevel::Log);
            let ret_dir = output_dir.clone();
            Ok(ret_dir)
        }
        Err(_) => {
            let save_fail = "Failed to save extracted ROM contents".to_string();
            log_write(save_fail.clone(), utils::LogLevel::Error);
            Err(RomExtractError::new(&save_fail))
        }
    }
}

// Only a placeholder for now
pub struct RomGenerateError{}

pub fn generate_rom(config: &str, new_nds_file: &str) -> Result<(),RomGenerateError> {
    log_write("This will take a long time (in debug mode)...", LogLevel::Debug);
    let Ok(rom) = Rom::load(&config, RomLoadOptions::default()) else {
        utils::log_write(format!("Failed to load directory '{}'",&config), utils::LogLevel::Error);
        return Err(RomGenerateError{});
    };
    log_write("Config processed successfully", LogLevel::Log);
    let Ok(raw_rom) = rom.build(None) else {
        utils::log_write("Failed to build ROM".to_string(), utils::LogLevel::Error);
        return Err(RomGenerateError{});
    };
    match raw_rom.save(new_nds_file) {
        Err(_) => {
            utils::log_write(format!("Failed to generate ROM '{}'",new_nds_file), utils::LogLevel::Error);
            Err(RomGenerateError{})
        }
        Ok(_) => {
            utils::log_write(format!("Generated ROM '{}' successfully",new_nds_file), utils::LogLevel::Log);
            Ok(())
        }
    }
}