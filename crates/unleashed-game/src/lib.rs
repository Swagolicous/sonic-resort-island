#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageId {
    WindmillIsleAct1,
}

impl StageId {
    pub const fn archive_stem(self) -> &'static str {
        match self {
            Self::WindmillIsleAct1 => "ActD_MykonosAct1",
        }
    }
}

pub mod camera;
pub mod sonic;
pub mod score;
