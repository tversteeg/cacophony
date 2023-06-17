use std::env;
use std::fs::File;
use std::io::Read;

use crate::{InputEvent, MidiBinding, MidiConn, NoteOn, QwertyBinding, KEYS};
use common::hashbrown::HashMap;
use common::ini::Ini;
use common::macroquad::input::*;
use common::State;
use std::str::FromStr;

const MAX_OCTAVE: u8 = 9;
/// Only these events are allowed during alphanumeric input.
const ALLOWED_DURING_ALPHANUMERIC_INPUT: [InputEvent; 12] = [
    InputEvent::Quit,
    InputEvent::AppTTS,
    InputEvent::StatusTTS,
    InputEvent::InputTTS,
    InputEvent::FileTTS,
    InputEvent::ToggleAlphanumericInput,
    InputEvent::UpDirectory,
    InputEvent::DownDirectory,
    InputEvent::SelectFile,
    InputEvent::NextPath,
    InputEvent::PreviousPath,
    InputEvent::CloseOpenFile,
];

/// Listens for user input from qwerty and MIDI devices and records the current input state.
#[derive(Default)]
pub struct Input {
    /// Events that began on this frame (usually due to a key press or MIDI controller message).
    events: Vec<InputEvent>,
    /// The MIDI connection.
    midi_conn: Option<MidiConn>,
    // Note-on MIDI messages. These will be sent immediately to the synthesizer to be played.
    pub play_now: Vec<[u8; 3]>,
    /// Note-on events that don't have corresponding off events.
    note_on_events: Vec<NoteOn>,
    /// Notes that were added after all note-off events are done.
    pub new_notes: Vec<[u8; 3]>,
    /// Input events generated by MIDI input.
    midi_events: HashMap<InputEvent, MidiBinding>,
    /// Input events generated by qwerty input.
    qwerty_events: HashMap<InputEvent, QwertyBinding>,
    /// The octave for qwerty input.
    qwerty_octave: u8,
    /// Was backspace pressed on this frame?
    backspace: bool,
    /// Characters pressed on this frame.
    pub pressed_chars: Vec<char>,
    /// Debug input events.
    debug_inputs: Vec<InputEvent>,
}

impl Input {
    pub fn new(config: &Ini) -> Self {
        // Get the audio connections.
        let midi_conn = MidiConn::new(config);

        // Get qwerty events.
        let mut qwerty_events: HashMap<InputEvent, QwertyBinding> = HashMap::new();
        // Get the qwerty input mapping.
        let keyboard_input = config.section(Some("QWERTY_BINDINGS")).unwrap();
        for kv in keyboard_input.iter() {
            let k_input = Input::parse_qwerty_binding(kv.0, kv.1);
            qwerty_events.insert(k_input.0, k_input.1);
        }

        // Get MIDI events.
        let mut midi_events: HashMap<InputEvent, MidiBinding> = HashMap::new();
        // Get the qwerty input mapping.
        let midi_input = config.section(Some("MIDI_BINDINGS")).unwrap();
        for kv in midi_input.iter() {
            let k_input = Input::parse_midi_binding(kv.0, kv.1);
            midi_events.insert(k_input.0, k_input.1);
        }

        let mut debug_inputs = vec![];
        if cfg!(debug_assertions) {
            let args: Vec<String> = env::args().collect();
            if args.len() >= 3 && args[1] == "--events" {
                match File::open(&args[2]) {
                    Ok(mut file) => {
                        let mut s = String::new();
                        file.read_to_string(&mut s).unwrap();
                        let lines = s.split('\n');
                        for line in lines {
                            match line.trim().parse::<InputEvent>() {
                                Ok(e) => debug_inputs.push(e),
                                Err(_) => panic!("Failed to parse {}", line),
                            }
                        }
                    }
                    Err(error) => panic!("Failed to open file {}: {}", &args[2], error),
                }
            }
        }

        Self {
            midi_conn,
            qwerty_events,
            midi_events,
            qwerty_octave: 4,
            debug_inputs,
            ..Default::default()
        }
    }

    pub fn update(&mut self, state: &State) {
        // Clear the old new notes.
        self.new_notes.clear();
        self.play_now.clear();

        // QWERTY INPUT.

        // Was backspace pressed?
        self.backspace = is_key_down(KeyCode::Backspace);
        // Get the pressed characters.
        self.pressed_chars.clear();
        while let Some(c) = get_char_pressed() {
            self.pressed_chars.push(c);
        }
        // Get all pressed keys.
        let pressed: Vec<KeyCode> = KEYS
            .iter()
            .filter(|&k| is_key_pressed(*k))
            .copied()
            .collect();
        // Get all held keys.
        let down: Vec<KeyCode> = KEYS.iter().filter(|&k| is_key_down(*k)).copied().collect();

        // Update the qwerty key bindings.
        self.qwerty_events
            .iter_mut()
            .for_each(|q| q.1.update(&pressed, &down, state.input.alphanumeric_input));
        // Get the key presses.
        let mut events: Vec<InputEvent> = self
            .qwerty_events
            .iter()
            .filter(|q| q.1.pressed)
            .map(|q| *q.0)
            .collect();
        // DEBUG.
        if cfg!(debug_assertions) && !&self.debug_inputs.is_empty() {
            let e = self.debug_inputs.remove(0);
            events.push(e);
        }

        // Qwerty note input.
        for event in events.iter() {
            match event {
                InputEvent::G => self.qwerty_note(7, state),
                InputEvent::FSharp => self.qwerty_note(6, state),
                InputEvent::F => self.qwerty_note(5, state),
                InputEvent::E => self.qwerty_note(4, state),
                InputEvent::DSharp => self.qwerty_note(3, state),
                InputEvent::D => self.qwerty_note(2, state),
                InputEvent::CSharp => self.qwerty_note(1, state),
                InputEvent::C => self.qwerty_note(0, state),
                InputEvent::B => self.qwerty_note(11, state),
                InputEvent::ASharp => self.qwerty_note(10, state),
                InputEvent::A => self.qwerty_note(9, state),
                InputEvent::GSharp => self.qwerty_note(8, state),
                // Octave up.
                InputEvent::OctaveUp => {
                    if self.qwerty_octave < MAX_OCTAVE {
                        self.qwerty_octave += 1;
                    }
                }
                // Octave down.
                InputEvent::OctaveDown => {
                    if self.qwerty_octave > 0 {
                        self.qwerty_octave -= 1;
                    }
                }
                _ => (),
            }
        }
        // Remove events during alphanumeric input.
        if state.input.alphanumeric_input {
            events.retain(|e| ALLOWED_DURING_ALPHANUMERIC_INPUT.contains(e));
        }
        self.events = events;

        // MIDI INPUT.
        if !state.input.alphanumeric_input {
            if let Some(midi_conn) = &mut self.midi_conn {
                // Poll for MIDI events.
                let midi = midi_conn.poll();
                // Append MIDI events.
                for mde in self.midi_events.iter_mut() {
                    if mde.1.update(midi) {
                        self.events.push(*mde.0);
                    }
                }

                // Get note-on and note-off events.
                let volume = state.input.volume.get() as u8;
                for midi in midi.iter() {
                    // Note-on.
                    if midi[0] >= 144 && midi[0] <= 159 {
                        // Set the volume.
                        let midi = if state.input.use_volume {
                            [midi[0], midi[1], volume]
                        } else {
                            *midi
                        };
                        // Remember the note-on for piano roll input.
                        if state.input.armed {
                            self.note_on_events.push(NoteOn::new(&midi));
                        }
                        // Copy this note to the immediate note-on array.
                        self.play_now.push(midi);
                    }
                    // Note-off.
                    if state.input.armed && midi[0] >= 128 && midi[0] <= 143 {
                        // Find the corresponding note.
                        for note_on in self.note_on_events.iter_mut() {
                            // Same key. Note-off.
                            if note_on.note[1] == midi[1] {
                                note_on.off = true;
                            }
                        }
                    }
                }
                // If all note-ons are off, add them to the `notes` buffer as notes.
                if !self.note_on_events.is_empty() && self.note_on_events.iter().all(|n| n.off) {
                    for note_on in self.note_on_events.iter() {
                        self.new_notes.push(note_on.note);
                    }
                    self.note_on_events.clear();
                }
            }
        }
    }

    /// Returns true if the event happened.
    pub fn happened(&self, event: &InputEvent) -> bool {
        self.events.contains(event)
    }

    /// Reads the qwerty and MIDI bindings for an event.
    pub fn get_bindings(
        &self,
        event: &InputEvent,
    ) -> (Option<&QwertyBinding>, Option<&MidiBinding>) {
        (self.qwerty_events.get(event), self.midi_events.get(event))
    }

    /// Modify a string with qwerty input from this frame. Allow alphanumeric input.
    pub fn modify_string_abc123(&self, string: &mut String) -> bool {
        self.modify_string(
            string,
            &self
                .pressed_chars
                .iter()
                .filter(|c| c.is_ascii_alphanumeric())
                .copied()
                .collect(),
        )
    }

    /// Modify a u32 value.
    pub fn modify_u32(&self, value: &mut u32) -> bool {
        self.modify_value(value, 0)
    }

    /// Modify a value with qwerty input from this frame. Allow numeric input.
    fn modify_value<T>(&self, value: &mut T, default_value: T) -> bool
    where
        T: ToString + FromStr,
    {
        // Convert the value to a string.
        let mut string = value.to_string();
        // Modify the string.
        let modified = self.modify_string(
            &mut string,
            &self
                .pressed_chars
                .iter()
                .filter(|c| c.is_ascii_digit())
                .copied()
                .collect(),
        );
        // Try to get a value.
        match T::from_str(string.as_str()) {
            Ok(v) => *value = v,
            Err(_) => *value = default_value,
        }
        modified
    }

    /// Modify a string with qwerty input from this frame.
    fn modify_string(&self, string: &mut String, chars: &Vec<char>) -> bool {
        // Delete the last character.
        if self.backspace {
            string.pop().is_some()
        // Add new characters.
        } else if !chars.is_empty() {
            for ch in chars.iter() {
                string.push(*ch);
            }
            true
        } else {
            false
        }
    }

    /// Parse a qwerty binding from a key-value pair of strings (i.e. from a config file).
    fn parse_qwerty_binding(key: &str, value: &str) -> (InputEvent, QwertyBinding) {
        match key.parse::<InputEvent>() {
            Ok(input_key) => (input_key, QwertyBinding::deserialize(value)),
            Err(error) => panic!("Invalid input key {}: {}", key, error),
        }
    }

    // Parse a MIDI binding from a key-value pair of strings (i.e. from a config file).
    fn parse_midi_binding(key: &str, value: &str) -> (InputEvent, MidiBinding) {
        match key.parse::<InputEvent>() {
            Ok(input_key) => (input_key, MidiBinding::deserialize(value)),
            Err(error) => panic!("Invalid input key {}: {}", key, error),
        }
    }

    /// Push a new note from qwerty input.
    fn qwerty_note(&mut self, note: u8, state: &State) {
        let pitch = (9 - self.qwerty_octave) * 12 + note;
        let note = [144, pitch, state.input.volume.get() as u8];
        if state.input.armed {
            self.new_notes.push(note);
        }
        self.play_now.push(note);
    }
}
