use std::{fs, io::Cursor, path::PathBuf};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use uuid::Uuid;

use crate::{engine::compression::segment_wrap, utils::{self, header_to_string, log_write, LogLevel}};

use super::Compilable;

/// CRSB (Course Binary)
#[derive(Clone,Debug)]
pub struct CourseInfo {
    pub level_map_data: Vec<CourseMapInfo>,
    pub src_filename: String,
    pub label: String
}
impl Default for CourseInfo {
    fn default() -> Self {
        Self {
            level_map_data: Vec::new(),
            src_filename: "ERROR".to_owned(),
            label: "DEFAULT".to_owned()
        }
    }
}
impl Compilable for CourseInfo {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u32::<LittleEndian>(self.level_map_data.len() as u32);
        for map_data in &self.level_map_data {
            let mut data_bytes = map_data.wrap();
            comp.append(&mut data_bytes);
        }
        // Pad to 4 bytes, all must be
        while comp.len() % 4 != 0 {
            comp.push(0x00);
        }
        comp
    }
}
impl CourseInfo {
    pub fn new(abs_path: &PathBuf, label: &String) -> Self {
        // It is uncompressed
        let file_bytes = fs::read(abs_path);
        if file_bytes.is_err() {
            utils::log_write(format!("Failed to read Course file: '{}'",file_bytes.unwrap_err()), utils::LogLevel::ERROR);
            return CourseInfo::default();
        }
        let file_bytes: Vec<u8> = file_bytes.unwrap();
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(&file_bytes);
        let file_header = rdr.read_u32::<LittleEndian>();
        if file_header.is_err() {
            utils::log_write(format!("Failed to read file header: '{}'",file_header.unwrap_err()), utils::LogLevel::ERROR);
            return CourseInfo::default();
        }
        let file_header = file_header.unwrap();
        if header_to_string(&file_header) != "CRSB" {
            utils::log_write("Course data header was not CRSB", utils::LogLevel::WARN);
        }
        // Okay, checks are out of the way. Lets start reading!
        let _crsb_internal_size = rdr.read_u32::<LittleEndian>().unwrap();
        let cscn_count: u32 = rdr.read_u32::<LittleEndian>().unwrap();
        // Time for the CSCN loop
        let mut cscn_vec: Vec<CourseMapInfo> = Vec::new();
        let mut cscn_index: u32 = 0;
        while cscn_index < cscn_count {
            // Begin reading a CSCN segment
            let cscn_header = rdr.read_u32::<LittleEndian>().unwrap();
            let cscn_header_string = header_to_string(&cscn_header);
            if cscn_header_string != "CSCN" {
                utils::log_write(format!("Wrong header, expected CSCN, got '{}'/0x{:08X}",cscn_header_string,&cscn_header), utils::LogLevel::WARN);
            }
            let _cscn_internal_size: u32 = rdr.read_u32::<LittleEndian>().unwrap();
            let cscn_entrance_count: u16 = rdr.read_u16::<LittleEndian>().unwrap();
            let mut cscn_entrance_vec: Vec<MapEntrance> = Vec::new();
            let cscn_exit_count: u8 = rdr.read_u8().unwrap();
            let mut cscn_exit_vec: Vec<MapExit> = Vec::new();
            let cscn_music_id: u8 = rdr.read_u8().unwrap();
            // 16 bytes are reserved for the file name, even if not filled
            let index_after = rdr.position() + 16;
            // Read the map file name
            let mpdz_name_noext:String = utils::read_c_string(&mut rdr);
            // Jumps ahead for padding (0x02033224 ?)
            rdr.set_position(index_after);
            // First up: entrance loop
            let mut entrance_index: u16 = 0;
            while entrance_index < cscn_entrance_count {
                let entrance_x: u16 = rdr.read_u16::<LittleEndian>().unwrap();
                let entrance_y: u16 = rdr.read_u16::<LittleEndian>().unwrap();
                let entrance_flags: u16 = rdr.read_u16::<LittleEndian>().unwrap();
                let entrance: MapEntrance = MapEntrance {
                    entrance_x, entrance_y, entrance_flags,
                    label: format!("Entrance 0x{:X}",entrance_index),
                    uuid: Uuid::new_v4()
                };
                cscn_entrance_vec.push(entrance); // Hand it over
                entrance_index += 1;
            }
            // Since entrance data is only 6 bytes... Not divisible by 4!
            while rdr.position() % 4 != 0 {
                let _ = rdr.read_u8();
            }
            // Exit loop time
            let mut exit_index: u8 = 0;
            while exit_index < cscn_exit_count {
                let exit_x: u16 = rdr.read_u16::<LittleEndian>().unwrap();
                //println!("exit_x = {:04X}",exit_x);
                let exit_y: u16 = rdr.read_u16::<LittleEndian>().unwrap();
                let exit_type: u16 = rdr.read_u16::<LittleEndian>().unwrap();
                let target_map_raw: u8 = rdr.read_u8().unwrap();
                let target_map_entrance_raw: u8 = rdr.read_u8().unwrap();
                //println!("target_map_entrance = {:02X}",&target_map_entrance);
                let exit: MapExit = MapExit {
                    exit_x, exit_y, exit_type, target_map_raw, target_map_entrance_raw,
                    label: format!("Exit 0x{:X}",exit_index), uuid: Uuid::new_v4(),
                    target_map: Uuid::nil(), target_map_entrance: Uuid::nil()
                };
                cscn_exit_vec.push(exit);
                exit_index += 1;
            }
            let cscn: CourseMapInfo = CourseMapInfo {
                map_music: cscn_music_id,
                map_filename_noext: mpdz_name_noext.clone(),
                map_entrances: cscn_entrance_vec,
                map_exits: cscn_exit_vec,
                label: format!("0x{:X}: {}",cscn_index,&mpdz_name_noext),
                uuid: Uuid::new_v4()
            };
            cscn_vec.push(cscn); // Move it in
            cscn_index += 1;
        } // CSCN Loop over
        let mut ret = CourseInfo {
            level_map_data: cscn_vec,
            src_filename: abs_path.to_str().unwrap_or("UNWRAP FAILURE").to_owned(),
            label: label.clone()
        };
        let _update_uuids_res = ret.update_exit_uuids();
        ret
    }

    pub fn wrap(&mut self) -> Vec<u8> {
        self.update_exit_indexes();
        let uncomped_bytes: Vec<u8> = self.compile();
        // SCEN files are not compressed, though sub-segments are
        segment_wrap(&uncomped_bytes, "CRSB".to_owned())
    }

    /// Update UUID lists from indexes, only do right after load
    fn update_exit_uuids(&mut self) -> Result<(),()> {
        log_write(format!("Updating Exit UUIDs for {}",self.src_filename), LogLevel::DEBUG);
        let maps_ro = self.level_map_data.clone();
        for map in &mut self.level_map_data {
            for exit in &mut map.map_exits {
                if exit.target_map_raw as usize >= maps_ro.len() {
                    log_write("Target Map Raw out of bounds!", LogLevel::ERROR);
                    return Err(());
                }
                let target_map = &maps_ro[exit.target_map_raw as usize];
                exit.target_map = target_map.uuid;
                if exit.target_map_entrance_raw as usize >= target_map.map_entrances.len() {
                    log_write("Target Map Entrance Raw out of bounds!", LogLevel::ERROR);
                    return Err(());
                }
                let target_map_entrance = &target_map.map_entrances[exit.target_map_entrance_raw as usize];
                exit.target_map_entrance = target_map_entrance.uuid;
            }
        }
        Ok(())
    }

    fn update_exit_indexes(&mut self) {
        log_write(format!("Updating Exit indexes for {}",self.src_filename), LogLevel::DEBUG);
        let maps_ro = self.level_map_data.clone();
        for map in &mut self.level_map_data {
            for exit in &mut map.map_exits {
                // Get Map Index
                let mut map_index: u8 = 0;
                for map in &maps_ro {
                    if map.uuid == exit.target_map {
                        break;
                    }
                    map_index += 1;
                }
                if map_index as usize >= maps_ro.len() {
                    log_write("map_index was out of bounds, setting to first", LogLevel::ERROR);
                    map_index = 0;
                }
                exit.target_map_raw = map_index;
                // Get Entrance Index
                let target_map = &maps_ro[map_index as usize];
                let ent_index = target_map.get_entrance_index(&exit.target_map_entrance);
                if ent_index.is_none() {
                    log_write(format!("No index found for entrance with uuid {}, setting to first",exit.target_map_entrance.to_string()), LogLevel::ERROR);
                    exit.target_map_entrance_raw = 0;
                } else {
                    let ent_index = ent_index.unwrap();
                    if ent_index as usize >= target_map.map_entrances.len() {
                        log_write("ent_index out of bounds, setting to first", LogLevel::ERROR);
                        exit.target_map_entrance_raw = 0;
                    } else {
                        exit.target_map_entrance_raw = ent_index;
                    }
                }
            }
        }
    }
}

/// CSCN (Info about map relative to the Level)
#[derive(Debug,Clone,PartialEq)]
pub struct CourseMapInfo {
    pub map_entrances: Vec<MapEntrance>,
    pub map_exits: Vec<MapExit>,
    pub map_music: u8,
    pub map_filename_noext: String,
    pub label: String,
    pub uuid: Uuid
}
impl Compilable for CourseMapInfo {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        // Entrance Count
        let _ = comp.write_u16::<LittleEndian>(self.map_entrances.len() as u16);
        // Exit Count
        let _ = comp.write_u8(self.map_exits.len() as u8);
        // Music ID
        let _ = comp.write_u8(self.map_music);
        // MPDZ name
        let mut str_vec = self.map_filename_noext.clone().into_bytes();
        comp.append(&mut str_vec);
        comp.push(0x00); // Null terminator
        // Why there are 8 bytes here, I know not
        for _spacer in 0..8 {
            comp.push(0x00);
        }
        // Now do the loops
        for enter in &self.map_entrances {
            let mut entrance = enter.compile();
            comp.append(&mut entrance);
        }
        while comp.len() % 4 != 0 {
            comp.push(0x00);
        }
        for exit in &self.map_exits {
            let mut exit_bytes = exit.compile();
            comp.append(&mut exit_bytes);
        }
        while comp.len() % 4 != 0 {
            comp.push(0x00);
        }
        comp
    }
}
impl CourseMapInfo {
    fn wrap(&self) -> Vec<u8> {
        let uncomped_bytes: Vec<u8> = self.compile();
        // SCEN files are not compressed, though sub-segments are
        segment_wrap(&uncomped_bytes, "CSCN".to_owned())
    }
    pub fn get_entrance_index(&self, entrance_uuid: &Uuid) -> Option<u8> {
        let mut index: u8 = 0;
        for enter in &self.map_entrances {
            if enter.uuid == *entrance_uuid {
                return Some(index);
            }
            index += 1;
        }
        Option::None
    }
    pub fn get_exit(&mut self, uuid: &Uuid) -> Option<&mut MapExit> {
        for exit in &mut self.map_exits {
            if exit.uuid == *uuid {
                return Some(exit);
            }
        }
        Option::None
    }
    pub fn get_entrance_mut(&mut self, entrance_uuid: &Uuid) -> Option<&mut MapEntrance> {
        for enter in &mut self.map_entrances {
            if enter.uuid == *entrance_uuid {
                return Some(enter);
            }
        }
        Option::None
    }
    pub fn get_entrance(&self, entrance_uuid: &Uuid) -> Option<&MapEntrance> {
        for enter in &self.map_entrances {
            if enter.uuid == *entrance_uuid {
                return Some(enter);
            }
        }
        Option::None
    }
}

#[derive(Debug,Clone,PartialEq)]
pub struct MapEntrance {
    pub entrance_x: u16,
    pub entrance_y: u16,
    pub entrance_flags: u16,
    pub label: String,
    pub uuid: Uuid
}
impl Compilable for MapEntrance {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u16::<LittleEndian>(self.entrance_x);
        let _ = comp.write_u16::<LittleEndian>(self.entrance_y);
        let _ = comp.write_u16::<LittleEndian>(self.entrance_flags);
        comp
    }
}

#[derive(Debug,Clone,PartialEq)]
pub struct MapExit {
    pub exit_x: u16,
    pub exit_y: u16,
    /// What triggers the exit, and what it looks like in some cases
    pub exit_type: u16,
    /// Only used for exporting
    pub target_map_raw: u8,
    /// Which map the exit will transition you to
    pub target_map: Uuid,
    // Only used for exporting
    pub target_map_entrance_raw: u8,
    /// Which entrance on that map you will go to
    pub target_map_entrance: Uuid,
    /// Only for UX purposes, no effect on game
    pub label: String,
    pub uuid: Uuid
}
impl Compilable for MapExit {
    fn compile(&self) -> Vec<u8> {
        let mut comp: Vec<u8> = vec![];
        let _ = comp.write_u16::<LittleEndian>(self.exit_x);
        let _ = comp.write_u16::<LittleEndian>(self.exit_y);
        let _ = comp.write_u16::<LittleEndian>(self.exit_type);
        let _ = comp.write_u8(self.target_map_raw);
        let _ = comp.write_u8(self.target_map_entrance_raw);
        comp
    }
}

// This is in LevelSelectData.h in the original Stork
// But confirm it anyway
pub fn exit_type_name(exit_type: u16) -> String {
    match exit_type {
        // No pipe sound
        0x0 => format!("{:X}: Walk Right (Silent)",exit_type),
        // No pipe sound
        0x1 => format!("{:X}: Walk Left (Silent)",exit_type),
        0x2 => format!("{:X}: Touch Pipe Up",exit_type),
        0x3 => format!("{:X}: Press Up Pipe?",exit_type),
        // Exactly what you'd expect
        0x4 => format!("{:X}: Press Down Pipe",exit_type),
        // Creates a graphic automatically
        0x5 => format!("{:X}: Blue Door Unlocked",exit_type),
        // Needs a key
        0x6 => format!("{:X}: Blue Door Locked",exit_type),
        // Creates a graphic automatically
        0x7 => format!("{:X}: Boss Door",exit_type),
        // Literally quits to the level select screen
        0x9 => format!("{:X}: Walk Right Map Quit",exit_type),
        // Simply go in the space to transition, usually jumping up into
        0xC => format!("{:X}: Area Trigger (Pipe)",exit_type),
        0xD => format!("{:X}: Minigame Exit",exit_type),
        _ => format!("Type 0x{:X}",exit_type)
    }
}

//    enum MapEntranceAnimation {
//         SPAWN_STATIC_RIGHT = 0x00, // If first map entrance, this is jump in from left. Pretty much always uses this
//         SPAWN_STATIC_LEFT = 0x01,  // If first map entrance, this is jump in from right. Unsure if used in base game
//         WALK_OUT_RIGHT = 0x02,
//         WALK_OUT_LEFT = 0x03,
//         SLOW_FALL_FACE_RIGHT = 0x04, // Slowly fall for a bit, then gravity resumes. "Big pipes" or ground holes
//         SLOW_FALL_FACE_LEFT = 0x05,
//         OUT_OF_PIPE_UPWARDS_SILENT_RIGHT = 0x06, // Pipe animation shown, but no sound places
//         OUT_OF_PIPE_UPWARDS_SILENT_LEFT = 0x07,
//         FLY_UP_RIGHT = 0x08, // Being shot up from underground, or going up to a cloud area usually
//         FLY_UP_LEFT = 0x09,
//         LOCKED_BLUE_DOOR_RIGHT = 0x0A,
//         OUT_OF_PIPE_UPWARDS_RIGHTFACE = 0x0B,
//         OUT_OF_PIPE_DOWNWARDS_RIGHTFACE = 0x0C,
//         OUT_OF_PIPE_UPWARDS_LEFTFACE = 0x0D,
//         OUT_OF_PIPE_DOWNWARDS_LEFTFACE = 0x0E,
//         OUT_OF_PIPE_RIGHTWARDS = 0x0F,
//         OUT_OF_PIPE_LEFTWARDS = 0x10,
//         LOCKED_BLUE_DOOR_LEFT = 0x11,
//         YOSHI_IS_INVISIBLE = 0x012, // Special case? Investigate in code
//         // Then 0x13 and 0x14 is spawn static left.. broken past 0x11 or 0x12 probably
//         NO_ENTRANCE = 0xff // Null value
//     };

// enum StartingDsScreen {
//     START_TOP_0 = 0,
//     START_TOP_1 = 1,
//     START_BOTTOM = 2,
//     START_TOP_2 = 3
// };
