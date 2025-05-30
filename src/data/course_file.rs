use std::{fs, io::Cursor, path::PathBuf};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use uuid::Uuid;

use crate::{engine::compression::segment_wrap, utils::{self, log_write, LogLevel}};

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
        let file_bytes = match fs::read(abs_path) {
            Err(error) => {
                utils::log_write(format!("Failed to read Course file: '{}'", error), utils::LogLevel::ERROR);
                return CourseInfo::default();
            }
            Ok(b) => b,
        };
        let mut rdr: Cursor<&Vec<u8>> = Cursor::new(&file_bytes);
        let file_header = match rdr.read_u32::<LittleEndian>() {
            Err(error) => {
                utils::log_write(format!("Failed to read file header: '{}'", error), utils::LogLevel::ERROR);
                return CourseInfo::default();
            }
            Ok(h) => h,
        };
        if utils::header_to_string(&file_header) != "CRSB" {
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
            let cscn_header_string = utils::header_to_string(&cscn_header);
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
        ret.update_exit_uuids();
        ret
    }

    pub fn wrap(&mut self) -> Vec<u8> {
        self.update_exit_indexes();
        let uncomped_bytes: Vec<u8> = self.compile();
        // SCEN files are not compressed, though sub-segments are
        segment_wrap(&uncomped_bytes, "CRSB".to_owned())
    }

    /// Update UUID lists from indexes
    pub fn update_exit_uuids(&mut self) {
        log_write(format!("Updating Exit UUIDs for {}",self.src_filename), LogLevel::DEBUG);
        let maps_ro = self.level_map_data.clone();
        for map in &mut self.level_map_data {
            for exit in &mut map.map_exits {
                if exit.target_map_raw as usize >= maps_ro.len() {
                    log_write("Target Map Raw out of bounds!", LogLevel::ERROR);
                    return;
                }
                let target_map = &maps_ro[exit.target_map_raw as usize];
                exit.target_map = target_map.uuid;
                if exit.target_map_entrance_raw as usize >= target_map.map_entrances.len() {
                    log_write("Target Map Entrance Raw out of bounds!", LogLevel::ERROR);
                    return;
                }
                let target_map_entrance = &target_map.map_entrances[exit.target_map_entrance_raw as usize];
                exit.target_map_entrance = target_map_entrance.uuid;
            }
        }
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
                if let Some(ent_index) = target_map.get_entrance_index(&exit.target_map_entrance) {
                    if ent_index as usize >= target_map.map_entrances.len() {
                        log_write("ent_index out of bounds, setting to first", LogLevel::ERROR);
                        exit.target_map_entrance_raw = 0;
                    } else {
                        exit.target_map_entrance_raw = ent_index;
                    }
                } else {
                    log_write(format!("No index found for entrance with uuid {}, setting to first",exit.target_map_entrance.to_string()), LogLevel::ERROR);
                    exit.target_map_entrance_raw = 0;
                }
            }
        }
    }

    pub fn fix_exits(&mut self) {
        // First, we fix from UUIDs
        // Those are what we use most, and what will be broken
        let map_data_ro = self.level_map_data.clone();
        for map in &mut self.level_map_data {
            for exit in &mut map.map_exits {
                // Fix target maps
                if let Some(exit_map_pos) = map_data_ro.iter().position(|x| x.uuid == exit.target_map) {
                    // Set the raw to correct
                    exit.target_map_raw = exit_map_pos as u8;
                    // Entrance MIGHT still be good
                } else {
                    // The map must no longer exist
                    exit.target_map_raw = 0;
                    // The only guaranteed entrance on index 0 map
                    exit.target_map_entrance_raw = 0;
                    // No need to fix entrance, we already set it to , go to next
                    log_write(format!("Target map was missing for {} - {}, setting to first",map.label,exit.label), LogLevel::DEBUG);
                    continue;
                }

                // Fix target entrances
                let target_map_data_ro = &map_data_ro[exit.target_map_raw as usize];
                if let Some(entrance_pos) = target_map_data_ro.map_entrances.iter().position(|y| y.uuid == exit.target_map_entrance) {
                    // Update raw to correct
                    exit.target_map_entrance_raw = entrance_pos as u8;
                } else {
                    // Entrance no longer exists, set it to guarantee
                    exit.target_map_entrance_raw = 0;
                    log_write(format!("Target entrance was missing for {} - {}, setting to first",map.label,exit.label), LogLevel::DEBUG);
                }
            }
        }
        // All of the Raw values are valid now
        self.update_exit_uuids();
    }

    pub fn add_template(&mut self, template_file: &str, template_folder: &PathBuf) {
        log_write(format!("Adding new template map: '{}'",template_file), LogLevel::LOG);
        let root_path = template_folder.parent().expect("Every possible path has a parent");
        let mut source_file_path = template_folder.clone();
        source_file_path.push(template_file);
        let exists_check = fs::exists(&source_file_path);
        let Ok(exists) = exists_check else {
            log_write(format!("source_file_path existence check failed: '{}'",exists_check.unwrap_err()), LogLevel::ERROR);
            return;
        };
        if !exists {
            log_write(format!("Template file '{}' does not exist", &source_file_path.display()), LogLevel::ERROR);
            return;
        }
        // The file path is valid
        let mut four_num: u32 = 0;
        loop {
            four_num += 1;
            let new_file_name = format!("{}{:04}.mpdz",&template_file[0..3],four_num);
            let new_path = utils::nitrofs_abs(&root_path.to_path_buf(), &new_file_name);
            let Ok(new_path_exists) = fs::exists(&new_path) else {
                log_write("New Template path existence check failed", LogLevel::ERROR);
                continue;
            };
            if !new_path_exists {
                // It's good! Copy it
                let copy_res = fs::copy(&source_file_path, &new_path);
                match copy_res {
                    Ok(_) => {
                        log_write(format!("Successfully copied '{}' to '{}'",source_file_path.display(),new_path.display()), LogLevel::LOG);
                        let file_name_noext = new_file_name.replace(".mpdz", "");
                        println!("file_name_noext: {}",file_name_noext);
                        // Now add the map to the data files
                        let new_course = CourseMapInfo::from_template(&file_name_noext);
                        self.fix_exits(); // Make sure everything is synced up before we add
                        self.level_map_data.push(new_course);
                        self.update_exit_uuids(); // Then fix the UUIDs (raws will be okay)
                        return;
                    },
                    Err(e) => {
                        log_write(format!("Error in template file copy: '{}'",e), LogLevel::ERROR);
                        return;
                    }
                }
            } // Otherwise, continue
        }
    }

    pub fn delete_map_info_by_index(&mut self, index: usize) -> bool {
        if index >= self.level_map_data.len() {
            log_write("Overflow in delete_map_info_by_index", LogLevel::ERROR);
            return false;
        }
        self.level_map_data.remove(index);
        self.fix_exits();
        true
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
        self.map_exits.iter_mut().find(|x| x.uuid == *uuid)
    }
    pub fn get_entrance_mut(&mut self, entrance_uuid: &Uuid) -> Option<&mut MapEntrance> {
        self.map_entrances.iter_mut().find(|e| e.uuid == *entrance_uuid)
    }
    pub fn get_entrance(&self, entrance_uuid: &Uuid) -> Option<&MapEntrance> {
        self.map_entrances.iter().find(|e| e.uuid == *entrance_uuid)
    }
    pub fn add_entrance(&mut self) -> Uuid {
        let new_index = self.map_entrances.len(); // Indexes start at 0
        let label = format!("Entrance 0x{:X}",new_index);
        let new_ent = MapEntrance {
            entrance_x: 0, entrance_y: 0,
            entrance_flags: 0x8009, // TODO: Better default?
            label, uuid: Uuid::new_v4()
        };
        let ret_uuid = new_ent.uuid;
        self.map_entrances.push(new_ent);
        ret_uuid
    }
    pub fn add_exit(&mut self) -> Uuid {
        let new_index = self.map_exits.len(); // So this is the next index
        let new_exit = MapExit {
            exit_x: 0,
            exit_y: 0,
            exit_type: 0,
            target_map_raw: 0xff,
            target_map: Uuid::nil(), // Fix this from course
            target_map_entrance_raw: 0xff,
            target_map_entrance: Uuid::nil(),
            label: format!("Exit 0x{:X}",new_index),
            uuid: Uuid::new_v4()
        };
        let ret_uuid = new_exit.uuid;
        self.map_exits.push(new_exit);
        ret_uuid
    }
    pub fn delete_exit(&mut self, exit_uuid: Uuid) -> bool {
        if let Some(pos) = self.map_exits.iter().position(|x| x.uuid == exit_uuid) {
            self.map_exits.remove(pos);
            log_write("Exit data deleted", LogLevel::DEBUG);
            true
        } else {
            log_write(format!("Failed to delete MapExit with UUID {}",exit_uuid), LogLevel::ERROR);
            false
        }
    }
    pub fn delete_entrance(&mut self, entrance_uuid: Uuid) -> bool {
        if let Some(pos) = self.map_entrances.iter().position(|x| x.uuid == entrance_uuid) {
            self.map_entrances.remove(pos);
            log_write("Entrance data deleted", LogLevel::DEBUG);
            true
        } else {
            log_write(format!("Failed to delete MapEntrance with UUID {}",entrance_uuid), LogLevel::ERROR);
            false
        }
    }

    pub fn from_template(name_no_ext: &str) -> Self {
        CourseMapInfo {
            map_entrances: vec![MapEntrance::default()],
            map_exits: vec![MapExit::default()],
            map_music: 0,
            map_filename_noext: name_no_ext.to_string(),
            label: name_no_ext.to_string(),
            uuid: Uuid::new_v4()
        }
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
impl Default for MapEntrance {
    fn default() -> Self {
        Self {
            entrance_x: 2, entrance_y: 2,
            entrance_flags: 0x8000, // 1-1
            label: format!("Entrance {:02X}",rand::random::<u8>()),
            uuid: Uuid::new_v4()
        }
    }
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
impl Default for MapExit {
    fn default() -> Self {
        Self {
            exit_x: 0x10, exit_y: 0x10,
            exit_type: 0x5, // Blue Door
            target_map_raw: 0, target_map: Uuid::nil(),
            target_map_entrance_raw: 0, target_map_entrance: Uuid::nil(),
            label: format!("Exit {:02X}",rand::random::<u8>()), uuid: Uuid::new_v4()
        }
    }
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
