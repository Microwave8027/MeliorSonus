use rkyv::{Archive, Deserialize, Serialize};

/// Standard musical dynamic classifications (ppp to fff).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicLevel {
    Pianississimo, // ppp
    Pianissimo,    // pp
    Piano,         // p
    MezzoPiano,    // mp
    MezzoForte,    // mf
    Forte,         // f
    Fortissimo,    // ff
    Fortississimo, // fff
    Other,
}

impl DynamicLevel {
    pub fn default_velocity(&self) -> u8 {
        match self {
            DynamicLevel::Pianississimo => 25,
            DynamicLevel::Pianissimo => 40,
            DynamicLevel::Piano => 55,
            DynamicLevel::MezzoPiano => 70,
            DynamicLevel::MezzoForte => 85,
            DynamicLevel::Forte => 100,
            DynamicLevel::Fortissimo => 115,
            DynamicLevel::Fortississimo => 127,
            DynamicLevel::Other => 80,
        }
    }

    pub fn to_audio_dynamic(&self) -> crate::audio_processing::instruments::notes::DynamicLevel {
        match self {
            DynamicLevel::Pianississimo => crate::audio_processing::instruments::notes::DynamicLevel::Pianississimo,
            DynamicLevel::Pianissimo => crate::audio_processing::instruments::notes::DynamicLevel::Pianissimo,
            DynamicLevel::Piano => crate::audio_processing::instruments::notes::DynamicLevel::Piano,
            DynamicLevel::MezzoPiano => crate::audio_processing::instruments::notes::DynamicLevel::MezzoPiano,
            DynamicLevel::MezzoForte => crate::audio_processing::instruments::notes::DynamicLevel::MezzoForte,
            DynamicLevel::Forte => crate::audio_processing::instruments::notes::DynamicLevel::Forte,
            DynamicLevel::Fortissimo => crate::audio_processing::instruments::notes::DynamicLevel::Fortissimo,
            DynamicLevel::Fortississimo => crate::audio_processing::instruments::notes::DynamicLevel::Fortississimo,
            DynamicLevel::Other => crate::audio_processing::instruments::notes::DynamicLevel::MezzoForte,
        }
    }
}

impl From<DynamicLevel> for crate::audio_processing::instruments::notes::DynamicLevel {
    fn from(d: DynamicLevel) -> Self {
        d.to_audio_dynamic()
    }
}
