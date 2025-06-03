// This is for data segments within SCEN files

use anmz::AnmzDataSegment;
use colz::CollisionData;
use imbz::ImbzData;
use imgb::ImgbData;
use info::ScenInfoData;
use mpbz::MapTileDataSegment;
use plan::AnimatedPaletteData;
use pltb::PltbData;
use rast::RastData;
use scrl::ScrollData;

pub mod info;
pub mod pltb;
pub mod mpbz;
pub mod anmz;
pub mod colz;
pub mod scrl;
pub mod imgb;
pub mod imbz;
pub mod plan;
pub mod rast;

#[derive(Clone,PartialEq,Debug)]
pub enum ScenSegmentWrapper {
    INFO(ScenInfoData),
    COLZ(CollisionData),
    PLTB(PltbData),
    SCRL(ScrollData),
    MPBZ(MapTileDataSegment),
    ANMZ(AnmzDataSegment),
    IMGB(ImgbData),
    IMBZ(ImbzData),
    PLAN(AnimatedPaletteData),
    RAST(RastData)
}

pub trait ScenSegment {
    /// Creates a byte vector, uncompressed
    fn compile(&self, info: Option<&ScenInfoData>) -> Vec<u8>;
    /// Creates a byte vector, with container and possible compression
    fn wrap(&self, info: Option<&ScenInfoData>) -> Vec<u8>;
    /// Get the header
    fn header(&self) -> String;
}

impl ScenSegment for ScenSegmentWrapper {
    fn compile(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        match self {
            Self::INFO(info_base) => info_base.compile(Option::None),
            Self::COLZ(colz) => colz.compile(info),
            Self::PLTB(pltb) => pltb.compile(info),
            Self::SCRL(scrl) => scrl.compile(info),
            Self::MPBZ(mpbz) => mpbz.compile(info),
            Self::ANMZ(anmz) => anmz.compile(info),
            Self::IMGB(imgb) => imgb.compile(info),
            Self::IMBZ(imbz) => imbz.compile(info),
            Self::PLAN(plan) => plan.compile(info),
            Self::RAST(rast) => rast.compile(info)
        }
    }

    fn wrap(&self, info: Option<&ScenInfoData>) -> Vec<u8> {
        match self {
            Self::INFO(info_base) => info_base.wrap(Option::None),
            Self::COLZ(colz) => colz.wrap(info),
            Self::PLTB(pltb) => pltb.wrap(info),
            Self::SCRL(scrl) => scrl.wrap(info),
            Self::MPBZ(mpbz) => mpbz.wrap(info),
            Self::ANMZ(anmz) => anmz.wrap(info),
            Self::IMGB(imgb) => imgb.wrap(info),
            Self::IMBZ(imbz) => imbz.wrap(info),
            Self::PLAN(plan) => plan.wrap(info),
            Self::RAST(rast) => rast.wrap(info)
        }
    }

    fn header(&self) -> String {
        match self {
            Self::INFO(info) => info.header(),
            Self::COLZ(colz) => colz.header(),
            Self::PLTB(pltb) => pltb.header(),
            Self::SCRL(scrl) => scrl.header(),
            Self::MPBZ(mpbz) => mpbz.header(),
            Self::ANMZ(anmz) => anmz.header(),
            Self::IMGB(imgb) => imgb.header(),
            Self::IMBZ(imbz) => imbz.header(),
            Self::PLAN(plan) => plan.header(),
            Self::RAST(rast) => rast.header()
        }
    }
}
