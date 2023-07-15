use crate::TtsString;
use common::config::parse;
use common::csv::Reader;
use common::hashbrown::hash_map::Entry;
use common::hashbrown::HashMap;
use common::ini::Ini;
use common::macroquad::input::KeyCode;
use common::{EditMode, Paths, PianoRollMode, Time, MIN_NOTE, PPQ_F, PPQ_U};
use input::{Input, InputEvent, QwertyBinding, KEYS};
use regex::Regex;

type RegexMap = HashMap<usize, Regex>;

const LANGUAGES: [&str; 1] = ["en"];
const KEYCODE_LOOKUPS: [&str; 121] = [
    "Space",
    "Apostrophe",
    "Comma",
    "Minus",
    "Period",
    "Slash",
    "Key0",
    "Key1",
    "Key2",
    "Key3",
    "Key4",
    "Key5",
    "Key6",
    "Key7",
    "Key8",
    "Key9",
    "Semicolon",
    "Equal",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "LeftBracket",
    "Backslash",
    "RightBracket",
    "GraveAccent",
    "World1",
    "World2",
    "Escape",
    "Enter",
    "Tab",
    "Backspace",
    "Insert",
    "Delete",
    "Right",
    "Left",
    "Down",
    "Up",
    "PageUp",
    "PageDown",
    "Home",
    "End",
    "CapsLock",
    "ScrollLock",
    "NumLock",
    "PrintScreen",
    "Pause",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
    "F25",
    "Kp0",
    "Kp1",
    "Kp2",
    "Kp3",
    "Kp4",
    "Kp5",
    "Kp6",
    "Kp7",
    "Kp8",
    "Kp9",
    "KpDecimal",
    "KpDivide",
    "KpMultiply",
    "KpSubtract",
    "KpAdd",
    "KpEnter",
    "KpEqual",
    "LeftShift",
    "LeftControl",
    "LeftAlt",
    "LeftSuper",
    "RightShift",
    "RightControl",
    "RightAlt",
    "RightSuper",
    "Menu",
    "Unknown",
];

type TextMap = HashMap<String, String>;

/// Localized text lookup.
pub struct Text {
    /// The text key-value map.
    text: TextMap,
    /// A map of key codes to spoken text.
    keycodes_spoken: HashMap<KeyCode, String>,
    /// A map of key codes to seen text.
    keycodes_seen: HashMap<KeyCode, String>,
    /// The text for each edit mode.
    edit_modes: HashMap<EditMode, String>,
    /// The text for each piano roll mode.
    piano_roll_modes: HashMap<PianoRollMode, String>,
    /// The name of each MIDI note.
    note_names: Vec<String>,
    /// Boolean dislay
    booleans: HashMap<bool, String>,
    /// Cached text-to-speech strings.
    tts_strings: HashMap<String, TtsString>,
    /// The regex used to find bindings.
    re_bindings: RegexMap,
    /// The regex used to find wildcard values.
    re_values: RegexMap,
}

impl Text {
    pub fn new(config: &Ini, paths: &Paths) -> Self {
        // Get the text language.
        let language: String = parse(config.section(Some("TEXT")).unwrap(), "language");
        // Find the column with the language.
        let column = LANGUAGES.iter().position(|&lang| lang == language).unwrap() + 1;
        // Get the text.
        let mut text = HashMap::new();
        // Read the .csv file.
        let mut reader = Reader::from_path(&paths.text_path).unwrap();
        for record in reader.records().filter(|r| r.is_ok()).flatten() {
            let key = record.get(0).unwrap().to_string();
            let value = record.get(column).unwrap().to_string();
            text.insert(key, value);
        }
        let note_names: Vec<String> = text
            .remove("NOTE_NAMES")
            .unwrap()
            .split(", ")
            .map(|s| s.to_string())
            .collect();
        let keycodes_spoken = Text::get_keycode_map(&text, true);
        let keycodes_seen = Text::get_keycode_map(&text, false);
        let edit_modes = Text::get_edit_mode_map(&text);
        let piano_roll_modes = Text::get_piano_roll_mode_map(&text);
        let mut booleans = HashMap::new();
        booleans.insert(true, text["TRUE"].clone());
        booleans.insert(false, text["FALSE"].clone());
        Self {
            text,
            keycodes_spoken,
            keycodes_seen,
            edit_modes,
            piano_roll_modes,
            note_names,
            booleans,
            tts_strings: HashMap::new(),
            re_bindings: RegexMap::new(),
            re_values: RegexMap::new(),
        }
    }

    /// Returns the text.
    pub fn get(&self, key: &str) -> String {
        match self.text.get(&key.to_string()) {
            Some(t) => t.clone(),
            None => panic!("Invalid text key {}", key),
        }
    }

    /// Returns the text. Fills in the values.
    pub fn get_with_values(&self, key: &str, values: &[&str]) -> String {
        match self.text.get(&String::from(key)) {
            Some(t) => {
                let mut text = t.clone();
                for (i, v) in values.iter().enumerate() {
                    let mut k: String = String::from("\\");
                    k.push_str(i.to_string().as_str());
                    let vv = v.to_string();
                    text = text.replace(&k, vv.as_str());
                }
                if text.contains('\\') {
                    println!("WARNING! Bad TTS text. {} {} {:?}", text, key, values);
                }
                text.replace("  ", " ")
            }
            None => panic!("Invalid text key {}", key),
        }
    }

    /// Returns the string version of a key code.
    pub fn get_keycode(&self, key: &KeyCode, spoken: bool) -> &str {
        match (if spoken {
            &self.keycodes_spoken
        } else {
            &self.keycodes_seen
        })
        .get(key)
        {
            Some(t) => t,
            None => panic!("Invalid key code {:?}", key),
        }
    }

    /// Returns the string version of a piano roll mode.
    pub fn get_piano_roll_mode(&self, mode: &PianoRollMode) -> String {
        match self.piano_roll_modes.get(mode) {
            Some(t) => t.clone(),
            None => panic!("Invalid piano roll mode {:?}", mode),
        }
    }

    /// Returns the string version of an edit mode.
    pub fn get_edit_mode(&self, mode: &EditMode) -> String {
        match self.edit_modes.get(mode) {
            Some(t) => t.clone(),
            None => panic!("Invalid edit mode {:?}", mode),
        }
    }

    /// Returns boolean text.
    pub fn get_boolean(&self, value: bool) -> String {
        self.booleans[&value].clone()
    }

    pub fn get_max_boolean_length(&self) -> usize {
        self.booleans
            .iter()
            .map(|kv| kv.1.chars().count())
            .max()
            .unwrap()
    }

    /// Converts a beat PPQ value into a time string.
    pub fn get_time(&self, ppq: u64, time: &Time) -> String {
        let duration = time.ppq_to_duration(ppq);
        let hours = duration.whole_hours();
        let minutes = duration.whole_minutes() - (hours * 60);
        let seconds = duration.whole_seconds() - (minutes * 60);
        // Include hours?
        match duration.whole_hours() > 0 {
            true => self.get_with_values(
                "TIME_TTS_HOURS",
                &[
                    hours.to_string().as_str(),
                    minutes.to_string().as_str(),
                    seconds.to_string().as_str(),
                ],
            ),
            false => self.get_with_values(
                "TIME_TTS",
                &[minutes.to_string().as_str(), seconds.to_string().as_str()],
            ),
        }
    }

    /// Returns a text-to-speech string of the `ppq` value.
    pub fn get_ppq_tts(&self, ppq: &u64) -> String {
        // This is a whole note.
        if ppq % PPQ_U == 0 {
            (ppq / PPQ_U).to_string()
        } else {
            match ppq {
                288 => self.get("FRACTION_TTS_ONE_AND_A_HALF"),
                96 => self.get("FRACTION_TTS_ONE_HALF"),
                64 => self.get("FRACTION_TTS_ONE_THIRD"),
                48 => self.get("FRACTION_TTS_ONE_FOURTH"),
                32 => self.get("FRACTION_TTS_ONE_SIXTH"),
                24 => self.get("FRACTION_TTS_ONE_EIGHTH"),
                12 => self.get("FRACTION_TTS_ONE_SIXTEENTH"),
                6 => self.get("FRACTION_TTS_ONE_THIRTY_SECOND"),
                other => format!("{:.2}", (*other as f32 / PPQ_F)),
            }
        }
    }

    /// Returns an error text-to-speech string.
    pub fn get_error(&self, error: &str) -> String {
        self.get_with_values("ERROR", &[error])
    }

    /// Returns the name of the note.
    pub fn get_note_name(&self, note: u8) -> String {
        self.note_names[(note - MIN_NOTE) as usize].clone()
    }

    /// Build a tooltip from a text lookup key and a list of events.
    ///
    /// - `key` The text lookup key, for example "TITLE_MAIN_MENU".
    /// - `events` An ordered list of input events. These will be inserted in the order that the binding wildcards are found.
    /// - `input` The input manager.
    ///
    /// Returns a tooltip `TtsString`.
    pub fn get_tooltip(&mut self, key: &str, events: &[InputEvent], input: &Input) -> TtsString {
        if let Entry::Occupied(o) = self.tts_strings.entry(key.to_string()) {
            o.get().clone()
        } else {
            let t = self.get_tooltip_with_values(key, events, &[], input);
            self.tts_strings.insert(key.to_string(), t);
            self.tts_strings[key].clone()
        }
    }

    /// Build a tooltip from a text lookup key and a list of events and another list of values.
    ///
    /// - `key` The text lookup key, for example "TITLE_MAIN_MENU".
    /// - `events` An ordered list of input events. The index is used to find the wildcard in the text, e.g. if the index is 0 then the wildcard is "\0".
    /// - `values` An ordered list of string values. The index is used to find the wildcard in the text, e.g. if the index is 0 then the wildcard is "%0".
    /// - `input` The input manager.
    ///
    /// Returns a list of text-to-speech strings.
    pub fn get_tooltip_with_values(
        &mut self,
        key: &str,
        events: &[InputEvent],
        values: &[&str],
        input: &Input,
    ) -> TtsString {
        // Get the string with the wildcards.
        let raw_string = self.get(key);
        let mut spoken = raw_string.clone();
        let mut seen = raw_string;
        let mut regexes = HashMap::new();
        // Iterate through each event.
        for (i, event) in events.iter().enumerate() {
            let regex = self.get_regex(&i, true);
            regexes.insert(i, regex.clone());
            // Get the key bindings.
            let bindings = input.get_bindings(event);
            // The replacement string.
            let mut spoken_replacement = vec![];
            let mut seen_replacement = vec![];
            let mut has_qwerty = false;
            // Get the qwerty binding.
            if let Some(qwerty) = bindings.0 {
                has_qwerty = true;
                // Add spoken mods.
                for m in self.get_mods(qwerty, true) {
                    spoken_replacement.push(m.to_string());
                }
                // Add seen mod tokens.
                for m in self.get_mods(qwerty, false) {
                    seen_replacement.push(m.to_string());
                }
                // Add spoken keys.
                for k in self.get_keys(qwerty, true) {
                    spoken_replacement.push(k.to_string());
                }
                // Add seen key tokens.
                for k in self.get_keys(qwerty, false) {
                    seen_replacement.push(k.to_string());
                }
            }
            // Get the MIDI binding.
            if let Some(midi) = bindings.1 {
                if has_qwerty {
                    // Or...
                    let or_str = self.get("OR").trim().to_string();
                    spoken_replacement.push(or_str.clone());
                    seen_replacement.push(or_str.clone());
                    // Get the MIDI binding.
                    let midi = match &midi.alias {
                        Some(alias) => alias.clone(),
                        None => self.get_with_values(
                            "MIDI_CONTROL",
                            &[&midi.bytes[0].to_string(), &midi.bytes[1].to_string()],
                        ),
                    };
                    spoken_replacement.push(midi.clone());
                    seen_replacement.push(midi);
                }
            }
            // Replace.
            spoken = regexes[&i]
                .replace(&spoken, &spoken_replacement.join(" "))
                .to_string();
            seen = regexes[&i]
                .replace(&seen, &seen_replacement.join(" "))
                .to_string();
        }
        // Iterate through each value.
        let mut regexes = HashMap::new();
        for (i, value) in values.iter().enumerate() {
            // Get the value regex.
            let regex = self.get_regex(&i, false);
            regexes.insert(i, regex.clone());
            // Replace the value wildcard.
            spoken = regex.replace(&spoken, *value).to_string();
            seen = regex.replace(&seen, *value).to_string();
        }
        TtsString { spoken, seen }
    }

    /// Returns a map of keycodes to displayable/sayable text (NOT string keys).
    fn get_keycode_map(text: &HashMap<String, String>, spoken: bool) -> HashMap<KeyCode, String> {
        let suffix = if spoken { "_SPOKEN" } else { "_SEEN" };
        let mut keycodes = HashMap::new();
        for (key, lookup) in KEYS.iter().zip(KEYCODE_LOOKUPS) {
            let mut lookup_key = lookup.to_string();
            lookup_key.push_str(suffix);
            keycodes.insert(*key, text[&lookup_key].clone());
        }
        keycodes
    }

    /// Returns a HashMap of the edit modes.
    fn get_edit_mode_map(text: &HashMap<String, String>) -> HashMap<EditMode, String> {
        let mut edit_modes = HashMap::new();
        edit_modes.insert(EditMode::Normal, text["EDIT_MODE_NORMAL"].clone());
        edit_modes.insert(EditMode::Quick, text["EDIT_MODE_QUICK"].clone());
        edit_modes.insert(EditMode::Precise, text["EDIT_MODE_PRECISE"].clone());
        edit_modes
    }

    /// Returns a HashMap of the piano roll modes.
    fn get_piano_roll_mode_map(text: &HashMap<String, String>) -> HashMap<PianoRollMode, String> {
        let mut piano_roll_modes = HashMap::new();
        piano_roll_modes.insert(PianoRollMode::Edit, text["PIANO_ROLL_MODE_EDIT"].clone());
        piano_roll_modes.insert(
            PianoRollMode::Select,
            text["PIANO_ROLL_MODE_SELECT"].clone(),
        );
        piano_roll_modes.insert(PianoRollMode::Time, text["PIANO_ROLL_MODE_TIME"].clone());
        piano_roll_modes.insert(PianoRollMode::View, text["PIANO_ROLL_MODE_VIEW"].clone());
        piano_roll_modes
    }

    /// Returns a regex that searches for wildcard `i`. Creates a regext if there is none.
    ///
    /// - `i` The wildcard value.
    /// - `bindings` If true, add a regex to `self.re_bindings`. If false add a regex to `self.re_values`.
    fn get_regex<'a>(&'a mut self, i: &usize, bindings: bool) -> &'a Regex {
        if bindings {
            Self::get_or_insert_regex(i, &mut self.re_bindings, r"\\")
        } else {
            Self::get_or_insert_regex(i, &mut self.re_values, "%")
        }
    }

    // Get or insert a regex in a HashMap.
    fn get_or_insert_regex<'a>(i: &usize, map: &'a mut RegexMap, prefix: &str) -> &'a Regex {
        map.entry(*i)
            .or_insert(Self::get_regex_from_index(i, prefix))
    }

    /// Returns a regex generated from index `i` and a string `prefix`.
    fn get_regex_from_index(i: &usize, prefix: &str) -> Regex {
        let mut r = prefix.to_string();
        r.push_str(&i.to_string());
        Regex::new(&r).unwrap()
    }

    /// Returns a qwerty binding's mods as strings.
    ///
    /// The strings may be different depending on the value of `spoken` i.e. whether this is meant to be spoken or seen.
    fn get_mods<'a>(&'a self, qwerty: &QwertyBinding, spoken: bool) -> Vec<&'a str> {
        qwerty
            .mods
            .iter()
            .map(|k| self.get_keycode(k, spoken))
            .collect::<Vec<&str>>()
    }

    /// Returns a qwerty binding's keys as strings.
    ///
    /// The strings may be different depending on the value of `spoken` i.e. whether this is meant to be spoken or seen.
    fn get_keys<'a>(&'a self, qwerty: &QwertyBinding, spoken: bool) -> Vec<&'a str> {
        qwerty
            .keys
            .iter()
            .map(|k| self.get_keycode(k, spoken))
            .collect::<Vec<&str>>()
    }
}
