#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterRange {
    pub hpf_cutoff_hz: f32,
    pub min_f0_hz: f32,
    pub max_f0_hz: f32,
    pub harmonic_ceiling_hz: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MpmConfig {
    pub power_threshold: f32,
    pub clarity_threshold: f32,
    pub is_strictly_monophonic: bool,
    pub min_poly_clarity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstrumentAcousticProfile {
    pub staccato_max_duration_sec: f32,
    pub tenuto_min_duration_sec: f32,
    pub rapid_rise_max_sec: f32,
    pub rapid_rise_min_slope: f32,
    pub mashed_crest_threshold_db: f32,
    pub mashed_sub_thump_dbfs: f32,
    pub flat_cents_threshold: i8,
    pub sharp_cents_threshold: i8,
    pub expected_centroid_min_hz: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Instrument {
    #[default]
    Generic,
    Piano,
    AcousticGuitar,
    ElectricGuitar,
    ElectricBass4,
    ElectricBass5,
    Violin,
    Viola,
    Cello,
    DoubleBass,
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
    VoiceSoprano,
    VoiceTenor,
    VoiceBass,
    Custom {
        hpf_cutoff_hz: f32,
        min_f0_hz: f32,
        max_f0_hz: f32,
        harmonic_ceiling_hz: f32,
    },
}

impl Instrument {
    pub fn mpm_config(&self) -> MpmConfig {
        match self {
            Instrument::Violin => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.70,
                is_strictly_monophonic: true,
                min_poly_clarity: 0.60,
            },
            Instrument::Viola => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.70,
                is_strictly_monophonic: true,
                min_poly_clarity: 0.60,
            },
            Instrument::Cello => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.65,
                is_strictly_monophonic: true,
                min_poly_clarity: 0.55,
            },
            Instrument::DoubleBass => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.60,
                is_strictly_monophonic: true,
                min_poly_clarity: 0.50,
            },
            Instrument::AcousticGuitar | Instrument::ElectricGuitar => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.70,
                is_strictly_monophonic: false,
                min_poly_clarity: 0.50,
            },
            Instrument::ElectricBass4 | Instrument::ElectricBass5 => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.65,
                is_strictly_monophonic: false,
                min_poly_clarity: 0.55,
            },
            Instrument::Flute | Instrument::ClarinetBb | Instrument::Oboe => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.75,
                is_strictly_monophonic: true,
                min_poly_clarity: 0.65,
            },
            Instrument::Bassoon | Instrument::AltoSax | Instrument::TenorSax => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.70,
                is_strictly_monophonic: true,
                min_poly_clarity: 0.60,
            },
            Instrument::TrumpetBb | Instrument::FrenchHorn | Instrument::TromboneTenor => {
                MpmConfig {
                    power_threshold: 0.00005,
                    clarity_threshold: 0.70,
                    is_strictly_monophonic: true,
                    min_poly_clarity: 0.60,
                }
            }
            Instrument::Tuba => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.65,
                is_strictly_monophonic: true,
                min_poly_clarity: 0.55,
            },
            Instrument::Piano => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.70,
                is_strictly_monophonic: false,
                min_poly_clarity: 0.50,
            },
            Instrument::VoiceSoprano | Instrument::VoiceTenor | Instrument::VoiceBass => {
                MpmConfig {
                    power_threshold: 0.00005,
                    clarity_threshold: 0.60,
                    is_strictly_monophonic: true,
                    min_poly_clarity: 0.50,
                }
            }
            Instrument::Generic | Instrument::Custom { .. } => MpmConfig {
                power_threshold: 0.00005,
                clarity_threshold: 0.65,
                is_strictly_monophonic: false,
                min_poly_clarity: 0.50,
            },
        }
    }

    pub fn filter_range(&self) -> FilterRange {
        match self {
            // Strings
            Instrument::Violin => FilterRange {
                hpf_cutoff_hz: 180.0,
                min_f0_hz: 196.0,
                max_f0_hz: 3136.0,
                harmonic_ceiling_hz: 18000.0,
            },
            Instrument::Viola => FilterRange {
                hpf_cutoff_hz: 120.0,
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
                harmonic_ceiling_hz: 12000.0,
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
                harmonic_ceiling_hz: 9000.0,
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
                max_f0_hz: 698.5,
                harmonic_ceiling_hz: 12000.0,
            },

            // Brass
            Instrument::TrumpetBb => FilterRange {
                hpf_cutoff_hz: 150.0,
                min_f0_hz: 164.8,
                max_f0_hz: 1046.5,
                harmonic_ceiling_hz: 14000.0,
            },
            Instrument::FrenchHorn => FilterRange {
                hpf_cutoff_hz: 55.0,
                min_f0_hz: 65.4,
                max_f0_hz: 880.0,
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
            Instrument::Piano => FilterRange {
                hpf_cutoff_hz: 22.0,
                min_f0_hz: 27.5,
                max_f0_hz: 4186.0,
                harmonic_ceiling_hz: 18000.0,
            },

            // Vocals
            Instrument::VoiceSoprano => FilterRange {
                hpf_cutoff_hz: 150.0,
                min_f0_hz: 180.0,
                max_f0_hz: 1200.0,
                harmonic_ceiling_hz: 18000.0,
            },
            Instrument::VoiceTenor => FilterRange {
                hpf_cutoff_hz: 70.0,
                min_f0_hz: 85.0,
                max_f0_hz: 650.0,
                harmonic_ceiling_hz: 15000.0,
            },
            Instrument::VoiceBass => FilterRange {
                hpf_cutoff_hz: 45.0,
                min_f0_hz: 55.0,
                max_f0_hz: 380.0,
                harmonic_ceiling_hz: 12000.0,
            },

            // Generic / Fallback
            Instrument::Generic => FilterRange {
                hpf_cutoff_hz: 30.0,
                min_f0_hz: 30.0,
                max_f0_hz: 4200.0,
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

    pub fn acoustic_profile(&self) -> InstrumentAcousticProfile {
        match self {
            Instrument::Piano => InstrumentAcousticProfile {
                staccato_max_duration_sec: 0.22,
                tenuto_min_duration_sec: 0.70,
                rapid_rise_max_sec: 0.015,
                rapid_rise_min_slope: 1.5,
                mashed_crest_threshold_db: 16.0,
                mashed_sub_thump_dbfs: -22.0,
                flat_cents_threshold: -8,
                sharp_cents_threshold: 8,
                expected_centroid_min_hz: 500.0,
            },
            Instrument::AcousticGuitar
            | Instrument::ElectricGuitar
            | Instrument::ElectricBass4
            | Instrument::ElectricBass5 => InstrumentAcousticProfile {
                staccato_max_duration_sec: 0.20,
                tenuto_min_duration_sec: 0.65,
                rapid_rise_max_sec: 0.012,
                rapid_rise_min_slope: 1.8,
                mashed_crest_threshold_db: 18.0,
                mashed_sub_thump_dbfs: -20.0,
                flat_cents_threshold: -12,
                sharp_cents_threshold: 12,
                expected_centroid_min_hz: 700.0,
            },
            Instrument::Violin | Instrument::Viola | Instrument::Cello | Instrument::DoubleBass => {
                InstrumentAcousticProfile {
                    staccato_max_duration_sec: 0.22,
                    tenuto_min_duration_sec: 0.90,
                    rapid_rise_max_sec: 0.060,
                    rapid_rise_min_slope: 0.5,
                    mashed_crest_threshold_db: 99.0, // Disabled: bowed instruments do not have keybed impact
                    mashed_sub_thump_dbfs: 0.0,
                    flat_cents_threshold: -15,
                    sharp_cents_threshold: 15,
                    expected_centroid_min_hz: 1000.0,
                }
            }
            Instrument::Flute
            | Instrument::ClarinetBb
            | Instrument::Oboe
            | Instrument::Bassoon
            | Instrument::AltoSax
            | Instrument::TenorSax => {
                InstrumentAcousticProfile {
                    staccato_max_duration_sec: 0.22,
                    tenuto_min_duration_sec: 0.80,
                    rapid_rise_max_sec: 0.040,
                    rapid_rise_min_slope: 0.7,
                    mashed_crest_threshold_db: 99.0, // Disabled: winds do not have keybed impact
                    mashed_sub_thump_dbfs: 0.0,
                    flat_cents_threshold: -14,
                    sharp_cents_threshold: 14,
                    expected_centroid_min_hz: 900.0,
                }
            }
            Instrument::TrumpetBb
            | Instrument::FrenchHorn
            | Instrument::TromboneTenor
            | Instrument::Tuba => {
                InstrumentAcousticProfile {
                    staccato_max_duration_sec: 0.24,
                    tenuto_min_duration_sec: 0.85,
                    rapid_rise_max_sec: 0.035,
                    rapid_rise_min_slope: 0.9,
                    mashed_crest_threshold_db: 99.0, // Disabled
                    mashed_sub_thump_dbfs: 0.0,
                    flat_cents_threshold: -15,
                    sharp_cents_threshold: 15,
                    expected_centroid_min_hz: 800.0,
                }
            }
            Instrument::VoiceSoprano | Instrument::VoiceTenor | Instrument::VoiceBass => {
                InstrumentAcousticProfile {
                    staccato_max_duration_sec: 0.25,
                    tenuto_min_duration_sec: 0.90,
                    rapid_rise_max_sec: 0.060,
                    rapid_rise_min_slope: 0.4,
                    mashed_crest_threshold_db: 99.0, // Disabled
                    mashed_sub_thump_dbfs: 0.0,
                    flat_cents_threshold: -18,
                    sharp_cents_threshold: 18,
                    expected_centroid_min_hz: 600.0,
                }
            }
            Instrument::Generic | Instrument::Custom { .. } => InstrumentAcousticProfile {
                staccato_max_duration_sec: 0.25,
                tenuto_min_duration_sec: 0.80,
                rapid_rise_max_sec: 0.020,
                rapid_rise_min_slope: 1.0,
                mashed_crest_threshold_db: 18.0,
                mashed_sub_thump_dbfs: -20.0,
                flat_cents_threshold: -10,
                sharp_cents_threshold: 10,
                expected_centroid_min_hz: 600.0,
            },
        }
    }
}
