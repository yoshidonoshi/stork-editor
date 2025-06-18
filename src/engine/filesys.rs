use std::{error::Error, fmt::Display, path::{Path, PathBuf}};

use ds_rom::rom::{raw, Rom, RomLoadOptions};
use crate::utils::{self, log_write, LogLevel};

/// Only a placeholder for now
#[derive(Debug, Clone)]
pub enum RomExtractError {
    FailedToOpenRom(String),
    FailedToExtractRom,
    FailedToSaveExtractedRom,

    LoadFileWithInvalidName(String),
    ProjectFolderDoesntExist,

    GenericFail,
}
impl Display for RomExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToOpenRom(path) => f.write_fmt(format_args!("Failed to open ROM file '{path}'")),
            Self::FailedToExtractRom => f.write_str("Failed to extract ROM contents"),
            Self::FailedToSaveExtractedRom => f.write_str("Failed to save extracted ROM contents"),

            Self::LoadFileWithInvalidName(path) => f.write_fmt(format_args!("Attempted to load file with invalid name: '{path}'")),
            Self::ProjectFolderDoesntExist => f.write_str("Project path failed existence check"),
            Self::GenericFail => f.write_str("Open ROM failed"),
        }
    }
}
impl Error for RomExtractError {}

pub fn extract_rom_files(nds_file: &Path, output_dir: &Path) -> Result<PathBuf,RomExtractError> {
    let Ok(raw_rom) = raw::Rom::from_file(nds_file) else {
        let open_fail = RomExtractError::FailedToOpenRom(nds_file.display().to_string());
        log_write(&open_fail, utils::LogLevel::Error);
        return Err(open_fail);
    };
    let Ok(rom) = Rom::extract(&raw_rom) else {
        let extract_err = RomExtractError::FailedToExtractRom;
        log_write(&extract_err, utils::LogLevel::Error);
        return Err(extract_err);
    };
    match rom.save(output_dir, None) {
        Ok(_) => {
            log_write(format!("ROM contents extracted to '{}' successfully", &output_dir.display()), utils::LogLevel::Log);
            let ret_dir = output_dir.to_path_buf();
            Ok(ret_dir)
        }
        Err(_) => {
            let save_fail = RomExtractError::FailedToSaveExtractedRom;
            log_write(&save_fail, utils::LogLevel::Error);
            Err(save_fail)
        }
    }
}

// Only a placeholder for now
pub struct RomGenerateError{}

pub fn generate_rom(config: &str, new_nds_file: &str) -> Result<(),RomGenerateError> {
    log_write("This will take a long time (in debug mode)...", LogLevel::Debug);
    let Ok(rom) = Rom::load(config, RomLoadOptions::default()) else {
        utils::log_write(format!("Failed to load directory '{config}'"), utils::LogLevel::Error);
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