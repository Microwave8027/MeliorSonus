use super::ending::RepeatEnding;
use super::jump::RepeatJump;
use super::repeat_range::{RepeatEnd, RepeatStart};
use rkyv::{Archive, Deserialize, Serialize};

/// Repeat variants container for start, end, volta endings, and jump marks.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum RepeatVariant {
    Start(RepeatStart),
    End(RepeatEnd),
    Ending(RepeatEnding),
    Jump(RepeatJump),
}

impl RepeatVariant {
    pub fn is_start(&self) -> bool {
        matches!(self, RepeatVariant::Start(_))
    }

    pub fn is_end(&self) -> bool {
        matches!(self, RepeatVariant::End(_))
    }

    pub fn is_ending(&self) -> bool {
        matches!(self, RepeatVariant::Ending(_))
    }

    pub fn is_jump(&self) -> bool {
        matches!(self, RepeatVariant::Jump(_))
    }

    pub fn as_start(&self) -> Option<&RepeatStart> {
        match self {
            RepeatVariant::Start(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_end(&self) -> Option<&RepeatEnd> {
        match self {
            RepeatVariant::End(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_ending(&self) -> Option<&RepeatEnding> {
        match self {
            RepeatVariant::Ending(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_jump(&self) -> Option<&RepeatJump> {
        match self {
            RepeatVariant::Jump(j) => Some(j),
            _ => None,
        }
    }
}
