use crate::audio_processing::instruments::notes::EndNote;
use std::error::Error;

/// Serializer for exporting practice runs to Standard MIDI Files (stubbed for `midly` crate).
pub struct MidiExporter;

impl MidiExporter {
    /// Serializes a slice of `EndNote` instances into Standard MIDI (`.mid`) file bytes.
    pub fn export_midi(_notes: &[EndNote], _bpm: f32) -> Result<Vec<u8>, Box<dyn Error>> {
        // Handoff stub for midly:
        // 1. Build Header (Format 0 or 1, PPQ timing).
        // 2. Map EndNote events (pitch, note_striked, note_duration, velocity) to NoteOn and NoteOff events with delta times.
        // 3. Serialize track chunk to byte buffer.
        Ok(Vec::new())
    }
}
