use super::{Density, OverlapMode};

impl Density {
    pub fn columns(self) -> usize {
        match self {
            Self::Compact => 6,
            Self::Regular => 5,
            Self::Comfy => 4,
        }
    }

    pub fn setting_index(self) -> usize {
        match self {
            Self::Compact => 0,
            Self::Regular => 1,
            Self::Comfy => 2,
        }
    }

    pub fn from_setting_index(index: usize) -> Self {
        match index {
            0 => Self::Compact,
            2 => Self::Comfy,
            _ => Self::Regular,
        }
    }
}

impl OverlapMode {
    pub fn setting_index(self) -> usize {
        match self {
            Self::Concurrent => 0,
            Self::Interrupt => 1,
        }
    }

    pub fn from_setting_index(index: usize) -> Self {
        match index {
            1 => Self::Interrupt,
            _ => Self::Concurrent,
        }
    }
}
