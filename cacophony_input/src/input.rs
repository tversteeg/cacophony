use crate::{MidiBinding, MidiConn, NoteOn, QwertyBinding, KEYS, InputEvent};
use cacophony_core::State;
use hashbrown::HashMap;
use ini::Ini;
use macroquad::input::*;

/// Listens for user input from qwerty and MIDI devices and records the current input state.
#[derive(Default)]
pub struct Input {
    /// Events that began on this frame (usually due to a key press or MIDI controller message).
    events: Vec<InputEvent>,
    /// The MIDI connection.
    midi_conn: Option<MidiConn>,
    // Note-on MIDI messages. These will be sent immediately to the synthesizer to be played.
    pub note_ons: Vec<[u8; 3]>,
    /// Note-on events that don't have corresponding off events.
    note_on_events: Vec<NoteOn>,
    /// Notes that were added after all note-off events are done.
    pub new_notes: Vec<[u8; 3]>,
    /// Input events generated by MIDI input.
    midi_events: HashMap<InputEvent, MidiBinding>,
    /// Input events generated by qwerty input.
    qwerty_events: HashMap<InputEvent, QwertyBinding>,
    /// Was backspace pressed on this frame?
    backspace: bool,
    /// Characters pressed on this frame.
    pub pressed_chars: Vec<char>,
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

        Self {
            midi_conn,
            qwerty_events,
            midi_events,
            ..Default::default()
        }
    }

    pub fn update(&mut self, state: &State) {
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
        self.events = self
            .qwerty_events
            .iter()
            .filter(|q| q.1.pressed)
            .map(|q| *q.0)
            .collect();

        // MIDI INPUT.
        let mut midi = vec![];
        if let Some(midi_conn) = &mut self.midi_conn {
            midi.extend(midi_conn.poll());

            // Append MIDI events.
            for mde in self.midi_events.iter_mut() {
                if mde.1.update(&midi) {
                    self.events.push(*mde.0);
                }
            }

            // Get note-on and note-off events.
            for midi in midi.iter() {
                // Note-on.
                if midi[0] >= 144 && midi[0] <= 159 {
                    if state.input.armed {
                        // Remember the note-on for piano roll input.
                        self.note_on_events.push(NoteOn::new(midi));
                    }
                    // Copy this note to the immediate note-on array.
                    self.note_ons.push(*midi);
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

    /// Returns true if the event happened.
    pub fn happened(&self, event: &InputEvent) -> bool {
        self.events.contains(event)
    }
    
    /// Reads the qwerty and MIDI bindings for an event.
    pub fn get_bindings(&self, event: &InputEvent) -> (Option<&QwertyBinding>, Option<&MidiBinding>) {
        (
            self.qwerty_events.get(event),
            self.midi_events.get(event),
        )
    }

    // Parse a qwerty binding from a key-value pair of strings (i.e. from a config file).
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
}
