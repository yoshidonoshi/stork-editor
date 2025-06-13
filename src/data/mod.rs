use crate::engine::compression::segment_wrap;

pub mod rarc;
pub mod segments;
pub mod types;
pub mod course_file;
pub mod mapfile;
pub mod backgrounddata;
pub mod scendata;
pub mod sprites;
pub mod grad;
pub mod area;
pub mod path;
pub mod alph;
pub mod blkz;
pub mod brak;

pub trait Compilable {
    /// This creates a byte vector readable by Yoshi's Island DS.
    /// No compilation or header wrapping!
    fn compile(&self) -> Vec<u8>;
}

/// Pseudo-Polymorphic compile segment
/// 
/// This is implemented in top level sugments such as SCEN and SETD, it's purpose
/// is to allow compilation via a simple loop instead of needing to have a custom
/// implementation for each
pub trait TopLevelSegment {
    /// Creates a byte vector that matches the "internal_data" of a segment, without compression
    fn compile(&self) -> Vec<u8>;
    /// Creates a byte vector, with header and possibly compression
    fn wrap(&self) -> Vec<u8>;
    /// Get the header as a String, for polymorphic purposes
    fn header(&self) -> String;
}

/// This makes it so there won't be broken levels upon save
#[derive(Clone,Debug,PartialEq)]
pub struct GenericTopLevelSegment {
    pub raw_bytes: Vec<u8>,
    pub header: String,
}

impl GenericTopLevelSegment {
    pub fn new(data: Vec<u8>, header: String) -> Self {
        Self { raw_bytes: data, header }
    }
}

impl TopLevelSegment for GenericTopLevelSegment {
    fn compile(&self) -> Vec<u8> {
        self.raw_bytes.clone()
    }

    fn wrap(&self) -> Vec<u8> {
        segment_wrap(self.compile(), self.header.clone())
    }

    fn header(&self) -> String {
        self.header.clone()
    }
}
