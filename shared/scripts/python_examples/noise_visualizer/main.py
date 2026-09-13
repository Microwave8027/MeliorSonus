import os
import sys
import time
import math
import struct
import wave
import shutil
from typing import Optional, List, Any

_DIR = os.path.dirname(os.path.abspath(__file__))
if hasattr(os, "add_dll_directory"):
    os.add_dll_directory(_DIR)
os.environ["PATH"] = _DIR + os.pathsep + os.environ.get("PATH", "")

import compose_app
from compose_app import *

# Improve representation of notes when printed inside a Python array/list
Notes.START.__repr__ = lambda self: f"Notes.START({self[0]!r})"
Notes.END.__repr__ = lambda self: f"Notes.END({self[0]!r})"
StartNote.__repr__ = (
    lambda self: f"StartNote(pitch='{self.pitch.name}', octave='{self.octave.name}', "
    f"velocity={self.velocity}, loudness_dbfs={self.loudness_dbfs:.1f})"
)
EndNote.__repr__ = (
    lambda self: f"EndNote(pitch='{self.pitch.name}', octave='{self.octave.name}', "
    f"duration={self.note_duration:.3f}s, articulation='{self.articulation.name}')"
)


class VisualizerCallback(CapturerCallback):
    """Callback receiver invoked by the Rust audio thread on errors or stream completion."""

    def __init__(self):
        self.is_complete = False
        self.error_message: Optional[str] = None

    def on_error(self, msg: str):
        print(f"\n[Audio Error]: {msg}")
        self.error_message = msg

    def on_complete(self):
        print("\n[Audio Complete]: Finished processing audio file.")
        self.is_complete = True


def resolve_default_wav_path() -> str:
    """Finds the default wav file in the assets directory."""
    assets_dir = os.path.join(_DIR, "assets")
    fur_elise = os.path.join(assets_dir, "fur elise (Ludwig van Beethoven).wav")
    if os.path.exists(fur_elise):
        return fur_elise

    if os.path.isdir(assets_dir):
        for f in os.listdir(assets_dir):
            if f.lower().endswith(".wav"):
                return os.path.join(assets_dir, f)

    return fur_elise


def save_pitches_to_txt(pitches: List[str], output_path: Optional[str] = None, delimiter: str = "\n"):
    """
    Saves a list of pitch strings to a .txt file.
    
    :param pitches: List of pitch strings (e.g. ['E', 'DS_EF', 'B'] or ['E5', 'D#5', 'B4'])
    :param output_path: Target .txt file path (defaults to assets/pitches.txt)
    :param delimiter: Separator between pitches ('\n' for one per line, ', ' for comma-separated)
    """
    if output_path is None:
        output_path = os.path.join(_DIR, "assets", "pitches.txt")

    # Ensure parent directory exists
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(delimiter.join(pitches) + "\n")

    print(f"[Saved] {len(pitches)} pitches written to: {output_path}")


def run_visualizer(wav_path: Optional[str] = None) -> List[Any]:
    if wav_path is None or not os.path.exists(wav_path):
        wav_path = resolve_default_wav_path()

    if not os.path.exists(wav_path):
        print(f"Error: WAV file not found at: {wav_path}")
        return []

    print("=" * 65)
    print("  MeliorSonus Audio Visualizer & Note Accumulator")
    print("=" * 65)
    print(f"Loading audio file: {wav_path}\n")

    accumulated_notes: List[str] = []
    callback = VisualizerCallback()
    capturer = AudioCapturer(wav_path, callback)

    print("Starting playback and DSP feature extraction (Press Ctrl+C to stop early)...")
    capturer.play()

    poll_interval = 0.01

    try:
        while not callback.is_complete and callback.error_message is None:
            note_event = capturer.get_notes()
            while note_event is not None:
                if isinstance(note_event, Notes.START):
                    # Append on note onset so each played pitch is recorded once
                    start_note = note_event[0]
                    pitch = start_note.pitch.name
                    octave = start_note.octave.name
                    vel = start_note.velocity
                    loudness = start_note.loudness_dbfs
                    accumulated_notes.append(pitch)
                    print(
                        f"  [NOTE ON]  | "
                        f"Pitch: {pitch:<5} Octave: {octave:<3} | "
                        f"Velocity: {vel:3} | Loudness: {loudness:6.1f} dBFS"
                    )
                elif isinstance(note_event, Notes.END):
                    end_note = note_event[0]
                    pitch = end_note.pitch.name
                    octave = end_note.octave.name
                    dur = end_note.note_duration
                    art = end_note.articulation.name
                    print(
                        f"  [NOTE OFF] | "
                        f"Pitch: {pitch:<5} Octave: {octave:<3} | "
                        f"Duration: {dur:5.3f}s | Articulation: {art}"
                    )
                note_event = capturer.get_notes()
            time.sleep(poll_interval)
    except KeyboardInterrupt:
        print("\n[Info] Interrupted by user.")
    finally:
        capturer.end()
        print("\n" + "=" * 65)
        print(f"Playback ended. Total pitches accumulated: {len(accumulated_notes)}")
        print("=" * 65)
        print("\nAccumulated Notes Array:")
        print(accumulated_notes)

        # Save pitches to .txt file
        output_txt = os.path.join(_DIR, "pitches.txt")
        save_pitches_to_txt(accumulated_notes, output_txt)

    return accumulated_notes


if __name__ == "__main__":
    target_wav = sys.argv[1] if len(sys.argv) > 1 else None
    run_visualizer(target_wav)


