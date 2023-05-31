use super::*;
use crate::panel::*;
use common::config::parse_fractions;
use common::ini::Ini;
use common::time::bar_to_samples;
use common::{Fraction, Index, Note, PianoRollMode, SelectMode, MAX_VOLUME};

/// The piano roll.
/// This is divided into different "modes" for convenience, where each mode is actually a panel.
pub struct PianoRollPanel {
    /// The edit mode.
    edit: Edit,
    /// The select mode.
    select: Select,
    /// The time mode.
    time: Time,
    /// The view mode.
    view: View,
    /// The beats that we can potentially input.
    beats: Vec<Fraction>,
    /// The index of the current beat.
    beat: Index,
    /// A buffer of copied notes.
    copied_notes: Vec<Note>,
}

impl PianoRollPanel {
    pub fn new(beat: &Fraction, config: &Ini) -> Self {
        let edit = Edit::new(config);
        let select = Select {};
        let time = Time::new(config);
        let view = View::new(config);
        // Load the beats.
        let section = config.section(Some("PIANO_ROLL")).unwrap();
        let mut beats = parse_fractions(section, "beats");
        // Is the input beat in the list?
        let beat_index = match beats.iter().position(|b| b == beat) {
            Some(position) => position,
            None => {
                beats.push(*beat);
                beats.len() - 1
            }
        };
        let beat = Index::new(beat_index, beats.len());
        Self {
            edit,
            select,
            time,
            view,
            beats,
            beat,
            copied_notes: vec![],
        }
    }

    /// Set the input beat.
    fn set_input_beat(&mut self, up: bool, state: &mut State) -> Option<UndoRedoState> {
        let s0 = state.clone();
        // Increment the beat.
        self.beat.increment(up);
        // Set the input beat.
        state.input.beat = self.beats[self.beat.get()];
        Some(UndoRedoState::from((s0, state)))
    }

    /// Set the piano roll mode.
    fn set_mode(mode: PianoRollMode, state: &mut State) -> Option<UndoRedoState> {
        let s0 = state.clone();
        state.piano_roll_mode = mode;
        Some(UndoRedoState::from((s0, state)))
    }

    /// Returns the text-to-speech string that will be said if there is no valid track.
    fn tts_no_track(text: &Text) -> String {
        text.get("PIANO_ROLL_PANEL_TTS_NO_TRACK")
    }

    /// Returns the sub-panel corresponding to the current piano roll mode.
    fn get_sub_panel<'a>(&'a self, state: &State) -> &'a dyn PianoRollSubPanel {
        match state.piano_roll_mode {
            PianoRollMode::Edit => &self.edit,
            PianoRollMode::Select => &self.select,
            PianoRollMode::Time => &self.time,
            PianoRollMode::View => &self.view,
        }
    }

    /// Copy the selected notes to the copy buffer.
    fn copy_notes(&mut self, state: &State) {
        if let Some(notes) = state.select_mode.get_notes(&state.music) {
            self.copied_notes = notes.iter().map(|&n| *n).collect()
        }
    }

    /// Delete notes from the track.
    fn delete_notes(state: &mut State) -> Option<UndoRedoState> {
        // Clone the state.
        let s0 = state.clone();
        if let Some(indices) = state.select_mode.get_note_indices() {
            if let Some(track) = state.music.get_selected_track_mut() {
                // Remove the notes.
                track.notes = track
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|n| !indices.contains(&n.0))
                    .map(|n| *n.1)
                    .collect();
                // Deselect.
                state.select_mode = match &state.select_mode {
                    SelectMode::Single(_) => SelectMode::Single(None),
                    SelectMode::Many(_) => SelectMode::Many(None),
                };
                // Return the undo state.
                return Some(UndoRedoState::from((s0, state)));
            }
        }
        None
    }
}

impl Panel for PianoRollPanel {
    fn update(
        &mut self,
        state: &mut State,
        conn: &mut Conn,
        input: &Input,
        tts: &mut TTS,
        text: &Text,
    ) -> Option<UndoRedoState> {
        // Do nothing.
        if state.music.selected.is_none() {
            None
        }
        // Add notes.
        else if state.input.armed && !input.new_notes.is_empty() {
            // Clone the state.
            let s0 = state.clone();
            let track = state.music.get_selected_track_mut().unwrap();
            match conn.state.programs.get(&track.channel) {
                Some(_) => {
                    // Get the notes.
                    let notes: Vec<Note> = input
                        .new_notes
                        .iter()
                        .map(|n| Note {
                            note: n[1],
                            velocity: n[2],
                            start: state.time.cursor,
                            duration: state.input.beat,
                        })
                        .collect();
                    // Add the notes.
                    track.notes.extend(notes.iter().copied());
                    // Move the cursor.
                    state.time.cursor += state.input.beat;
                    Some(UndoRedoState::from((s0, state)))
                }
                None => None,
            }
        }
        // Status TTS.
        else if input.happened(&InputEvent::StatusTTS) {
            let s = match state.music.get_selected_track() {
                Some(track) => match conn.state.programs.get(&track.channel) {
                    Some(_) => {
                        // The piano roll mode.
                        let mut s = text.get_with_values(
                            "PIANO_ROLL_PANEL_STATUS_TTS_PIANO_ROLL_MODE",
                            &[&text.get_piano_roll_mode(&state.piano_roll_mode)],
                        );
                        s.push(' ');
                        match state.input.armed {
                            // The beat and the volume.
                            true => {
                                let beat = text.get_fraction_tts(&state.input.beat);
                                let v = state.input.volume.get().to_string();
                                let volume = if state.input.use_volume {
                                    v
                                } else {
                                    text.get_with_values(
                                        "PIANO_ROLL_PANEL_STATUS_TTS_VOLUME",
                                        &[&v],
                                    )
                                };
                                s.push_str(&text.get_with_values(
                                    "PIANO_ROLL_PANEL_STATUS_TTS_ARMED",
                                    &[&beat, &volume],
                                ));
                            }
                            // Not armed.
                            false => s.push_str(&text.get("PIANO_ROLL_PANEL_STATUS_TTS_NOT_ARMED")),
                        }
                        // Piano role mode.
                        s.push(' ');
                        s.push_str(&text.get_with_values(
                            "PIANO_ROLL_PANEL_STATUS_TTS_PIANO_ROLL_MODE",
                            &[&text.get_piano_roll_mode(&state.piano_roll_mode)],
                        ));
                        // Panel-specific status.
                        s.push(' ');
                        s.push_str(&self.get_sub_panel(state).get_status_tts(state, text));
                        s
                    }
                    None => PianoRollPanel::tts_no_track(text),
                },
                None => PianoRollPanel::tts_no_track(text),
            };
            tts.say(&s);
            None
        }
        // Input TTS.
        else if input.happened(&InputEvent::InputTTS) {
            let s = match state.music.get_selected_track() {
                Some(track) => match conn.state.programs.get(&track.channel) {
                    // Here we go...
                    Some(_) => {
                        let mut s = vec![get_tooltip(
                            "PIANO_ROLL_PANEL_INPUT_TTS_PLAY",
                            &[InputEvent::PlayStop],
                            input,
                            text,
                        )];
                        // Armed state, beat, volume.
                        match state.input.armed {
                            true => {
                                s.push(get_tooltip(
                                    "PIANO_ROLL_PANEL_INPUT_TTS_ARMED",
                                    &[
                                        InputEvent::Arm,
                                        InputEvent::InputBeatLeft,
                                        InputEvent::InputBeatRight,
                                    ],
                                    input,
                                    text,
                                ));
                                match state.input.use_volume {
                                    true => s.push(get_tooltip(
                                        "PIANO_ROLL_PANEL_INPUT_TTS_DO_NOT_USE_VOLUME",
                                        &[
                                            InputEvent::DecreaseInputVolume,
                                            InputEvent::IncreaseInputVolume,
                                            InputEvent::ToggleInputVolume,
                                        ],
                                        input,
                                        text,
                                    )),
                                    false => s.push(get_tooltip(
                                        "PIANO_ROLL_PANEL_INPUT_TTS_USE_VOLUME",
                                        &[InputEvent::ToggleInputVolume],
                                        input,
                                        text,
                                    )),
                                }
                            }
                            false => s.push(get_tooltip(
                                "PIANO_ROLL_PANEL_INPUT_TTS_NOT_ARMED",
                                &[InputEvent::Arm],
                                input,
                                text,
                            )),
                        }
                        // Change the mode.
                        s.push(get_tooltip(
                            "PIANO_ROLL_PANEL_INPUT_TTS_MODES",
                            &[
                                InputEvent::PianoRollSetTime,
                                InputEvent::PianoRollSetView,
                                InputEvent::PianoRollSetSelect,
                                InputEvent::PianoRollSetEdit,
                            ],
                            input,
                            text,
                        ));
                        // Cut, copy.
                        let selected_some = state.select_mode.get_note_indices().is_some();
                        if selected_some {
                            s.push(get_tooltip(
                                "PIANO_ROLL_PANEL_INPUT_TTS_COPY_CUT",
                                &[InputEvent::CopyNotes, InputEvent::CutNotes],
                                input,
                                text,
                            ));
                        }
                        // Paste.
                        if !self.copied_notes.is_empty() {
                            s.push(get_tooltip(
                                "PIANO_ROLL_PANEL_INPUT_TTS_PASTE",
                                &[InputEvent::PasteNotes],
                                input,
                                text,
                            ));
                        }
                        // Delete.
                        if selected_some {
                            s.push(get_tooltip(
                                "PIANO_ROLL_PANEL_INPUT_TTS_DELETE",
                                &[InputEvent::DeleteNotes],
                                input,
                                text,
                            ));
                        }
                        // Sub-panel inputs.
                        s.push(self.get_sub_panel(state).get_input_tts(state, input, text));
                        s.join(" ")
                    }
                    None => PianoRollPanel::tts_no_track(text),
                },
                None => PianoRollPanel::tts_no_track(text),
            };
            tts.say(&s);
            None
        }
        // Play and stop music.
        else if input.happened(&InputEvent::PlayStop) {
            match conn.state.time.music {
                // Stop playing.
                true => conn.send(vec![Command::StopMusic]),
                false => {
                    // Get all tracks that can play music.
                    let tracks = match state.music.midi_tracks.iter().find(|t| t.solo) {
                        // Only include the solo track.
                        Some(solo) => vec![solo],
                        // Only include unmuted tracks.
                        None => state.music.midi_tracks.iter().filter(|t| !t.mute).collect(),
                    };
                    if !tracks.is_empty() {
                        // Get the bpm.
                        let bpm = state.music.bpm;
                        let t0 = bar_to_samples(&state.time.playback, bpm);
                        // Get the commands to start playing music.
                        let mut commands = vec![Command::PlayMusic { time: t0 }];
                        let max_gain = MAX_VOLUME as f64;
                        for track in tracks.iter() {
                            let gain = track.gain as f64 / max_gain;
                            // Add the note.
                            for note in track
                                .notes
                                .iter()
                                .filter(|n| n.start >= state.time.playback)
                            {
                                let start = bar_to_samples(&note.start, bpm) - t0;
                                let duration = bar_to_samples(&note.duration, bpm);
                                let velocity = (note.velocity as f64 * gain) as u8;
                                commands.push(Command::NoteOnAt {
                                    channel: track.channel,
                                    key: note.note,
                                    velocity,
                                    time: start,
                                    duration,
                                });
                            }
                        }
                        // We have some note-on commands.
                        if commands.len() > 1 {
                            conn.send(commands);
                        }
                    }
                }
            }
            None
        }
        // Copy notes.
        else if input.happened(&InputEvent::CopyNotes) {
            self.copy_notes(state);
            None
        }
        // Cut notes.
        else if input.happened(&InputEvent::CutNotes) {
            // Copy.
            self.copy_notes(state);
            // Delete.
            PianoRollPanel::delete_notes(state)
        }
        // Delete notes.
        else if input.happened(&InputEvent::DeleteNotes) {
            PianoRollPanel::delete_notes(state)
        }
        // Paste notes.
        else if input.happened(&InputEvent::PasteNotes) {
            if !self.copied_notes.is_empty() {
                // Clone the state.
                let s0 = state.clone();
                if let Some(track) = state.music.get_selected_track_mut() {
                    // Get the minimum start time.
                    let min_time = self
                        .copied_notes
                        .iter()
                        .min_by(|a, b| a.start.cmp(&b.start))
                        .unwrap()
                        .start;
                    // Get the time delta.
                    let dt = min_time - state.time.cursor;
                    // Clone the copied notes.
                    let mut notes = self.copied_notes.to_vec();
                    // Adjust the start time.
                    notes.iter_mut().for_each(|n| n.start -= dt);
                    // Add the notes.
                    track.notes.append(&mut notes);
                    // Return the undo state.
                    Some(UndoRedoState::from((s0, state)))
                } else {
                    None
                }
            } else {
                None
            }
        }
        // Toggle arm.
        else if input.happened(&InputEvent::Arm) {
            let s0 = state.clone();
            state.input.armed = !state.input.armed;
            Some(UndoRedoState::from((s0, state)))
        }
        // Set the input beat.
        else if input.happened(&InputEvent::InputBeatLeft) {
            self.set_input_beat(false, state)
        } else if input.happened(&InputEvent::InputBeatRight) {
            self.set_input_beat(true, state)
        }
        // Set the mode.
        else if input.happened(&InputEvent::PianoRollSetEdit) {
            PianoRollPanel::set_mode(PianoRollMode::Edit, state)
        } else if input.happened(&InputEvent::PianoRollSetSelect) {
            PianoRollPanel::set_mode(PianoRollMode::Select, state)
        } else if input.happened(&InputEvent::PianoRollSetTime) {
            PianoRollPanel::set_mode(PianoRollMode::Time, state)
        } else if input.happened(&InputEvent::PianoRollSetView) {
            PianoRollPanel::set_mode(PianoRollMode::View, state)
        }
        // Sub-panel actions.
        else {
            let mode = state.piano_roll_mode;
            match mode {
                PianoRollMode::Edit => self.edit.update(state, conn, input, tts, text),
                PianoRollMode::Select => self.select.update(state, conn, input, tts, text),
                PianoRollMode::Time => self.time.update(state, conn, input, tts, text),
                PianoRollMode::View => self.view.update(state, conn, input, tts, text),
            }
        }
    }
}
