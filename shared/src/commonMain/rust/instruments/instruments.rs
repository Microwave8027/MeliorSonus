#[derive(uniffi::Enum, Debug, Clone, Copy)]
pub enum Instrument {
    Violin,
    Viola,
    Cello,
    DoubleBass,
    AcousticGuitar,
    ElectricGuitar,
    ElectricBass4,
    ElectricBass5,
    Flute,
    ClarinetBb,
    Oboe,
    Bassoon,
    AltoSax,
    TenorSax,
    TrumpetBb,
    FrenchHorn,
    TromboneTenor,
    Tuba,
    GrandPiano,
    VoiceSoprano,
    VoiceTenor,
    VoiceBass,
    Generic,
    Custom {
        hpf_cutoff_hz: f32,
        min_f0_hz: f32,
        max_f0_hz: f32,
        harmonic_ceiling_hz: f32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct FilterRange {
    pub hpf_cutoff_hz: f32,
    pub min_f0_hz: f32,
    pub max_f0_hz: f32,
    pub harmonic_ceiling_hz: f32,
}

impl Instrument {
    pub fn filter_range(&self) -> FilterRange {
        match self {
            // Strings
            Instrument::Violin => FilterRange {
                hpf_cutoff_hz: 175.0,
                min_f0_hz: 196.0,
                max_f0_hz: 2637.0,
                harmonic_ceiling_hz: 18000.0,
            },
            Instrument::Viola => FilterRange {
                hpf_cutoff_hz: 115.0,
                min_f0_hz: 130.8,
                max_f0_hz: 1318.5,
                harmonic_ceiling_hz: 15000.0,
            },
            Instrument::Cello => FilterRange {
                hpf_cutoff_hz: 55.0,
                min_f0_hz: 65.4,
                max_f0_hz: 1046.5,
                harmonic_ceiling_hz: 12000.0,
            },
            Instrument::DoubleBass => FilterRange {
                hpf_cutoff_hz: 35.0,
                min_f0_hz: 41.2,
                max_f0_hz: 392.0,
                harmonic_ceiling_hz: 8000.0,
            },
            Instrument::AcousticGuitar => FilterRange {
                hpf_cutoff_hz: 70.0,
                min_f0_hz: 82.4,
                max_f0_hz: 987.8,
                harmonic_ceiling_hz: 15000.0,
            },
            Instrument::ElectricGuitar => FilterRange {
                hpf_cutoff_hz: 70.0,
                min_f0_hz: 82.4,
                max_f0_hz: 1318.5,
                harmonic_ceiling_hz: 8000.0,
            },
            Instrument::ElectricBass4 => FilterRange {
                hpf_cutoff_hz: 35.0,
                min_f0_hz: 41.2,
                max_f0_hz: 392.0,
                harmonic_ceiling_hz: 7000.0,
            },
            Instrument::ElectricBass5 => FilterRange {
                hpf_cutoff_hz: 25.0,
                min_f0_hz: 30.9,
                max_f0_hz: 392.0,
                harmonic_ceiling_hz: 7000.0,
            },

            // Woodwinds
            Instrument::Flute => FilterRange {
                hpf_cutoff_hz: 220.0,
                min_f0_hz: 246.9,
                max_f0_hz: 2349.3,
                harmonic_ceiling_hz: 16000.0,
            },
            Instrument::ClarinetBb => FilterRange {
                hpf_cutoff_hz: 130.0,
                min_f0_hz: 146.8,
                max_f0_hz: 1760.0,
                harmonic_ceiling_hz: 15000.0,
            },
            Instrument::Oboe => FilterRange {
                hpf_cutoff_hz: 205.0,
                min_f0_hz: 233.1,
                max_f0_hz: 1568.0,
                harmonic_ceiling_hz: 16000.0,
            },
            Instrument::Bassoon => FilterRange {
                hpf_cutoff_hz: 50.0,
                min_f0_hz: 58.3,
                max_f0_hz: 622.3,
                harmonic_ceiling_hz: 10000.0,
            },
            Instrument::AltoSax => FilterRange {
                hpf_cutoff_hz: 120.0,
                min_f0_hz: 138.6,
                max_f0_hz: 880.0,
                harmonic_ceiling_hz: 14000.0,
            },
            Instrument::TenorSax => FilterRange {
                hpf_cutoff_hz: 90.0,
                min_f0_hz: 103.8,
                max_f0_hz: 659.3,
                harmonic_ceiling_hz: 13000.0,
            },

            // Brass
            Instrument::TrumpetBb => FilterRange {
                hpf_cutoff_hz: 145.0,
                min_f0_hz: 164.8,
                max_f0_hz: 1174.7,
                harmonic_ceiling_hz: 16000.0,
            },
            Instrument::FrenchHorn => FilterRange {
                hpf_cutoff_hz: 52.0,
                min_f0_hz: 61.7,
                max_f0_hz: 698.5,
                harmonic_ceiling_hz: 12000.0,
            },
            Instrument::TromboneTenor => FilterRange {
                hpf_cutoff_hz: 70.0,
                min_f0_hz: 82.4,
                max_f0_hz: 698.5,
                harmonic_ceiling_hz: 10000.0,
            },
            Instrument::Tuba => FilterRange {
                hpf_cutoff_hz: 30.0,
                min_f0_hz: 36.7,
                max_f0_hz: 349.2,
                harmonic_ceiling_hz: 6000.0,
            },

            // Keyboard / Piano
            Instrument::GrandPiano => FilterRange {
                hpf_cutoff_hz: 22.0,
                min_f0_hz: 27.5,
                max_f0_hz: 4186.0,
                harmonic_ceiling_hz: 18000.0,
            },

            // Vocals
            Instrument::VoiceSoprano => FilterRange {
                hpf_cutoff_hz: 220.0,
                min_f0_hz: 261.6,
                max_f0_hz: 1046.5,
                harmonic_ceiling_hz: 18000.0,
            },
            Instrument::VoiceTenor => FilterRange {
                hpf_cutoff_hz: 115.0,
                min_f0_hz: 130.8,
                max_f0_hz: 523.3,
                harmonic_ceiling_hz: 15000.0,
            },
            Instrument::VoiceBass => FilterRange {
                hpf_cutoff_hz: 70.0,
                min_f0_hz: 82.4,
                max_f0_hz: 329.6,
                harmonic_ceiling_hz: 12000.0,
            },

            // Generic / Fallback
            Instrument::Generic => FilterRange {
                hpf_cutoff_hz: 30.0,
                min_f0_hz: 30.0,
                max_f0_hz: 4000.0,
                harmonic_ceiling_hz: 20000.0,
            },

            // Custom
            Instrument::Custom {
                hpf_cutoff_hz,
                min_f0_hz,
                max_f0_hz,
                harmonic_ceiling_hz,
            } => FilterRange {
                hpf_cutoff_hz: *hpf_cutoff_hz,
                min_f0_hz: *min_f0_hz,
                max_f0_hz: *max_f0_hz,
                harmonic_ceiling_hz: *harmonic_ceiling_hz,
            },
        }
    }
}
