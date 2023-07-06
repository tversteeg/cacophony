use audio::exporter::Exporter;
use audio::*;
use common::serde_json::{from_str, to_string, Error};
use common::{PathsState, State};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

const READ_ERROR: &str = "Error reading file: ";
const WRITE_ERROR: &str = "Error writing file: ";

/// Serializable save data.
#[derive(Deserialize, Serialize)]
pub(crate) struct Save {
    state: State,
    synth_state: SynthState,
    paths_state: PathsState,
    exporter: Exporter,
}

impl Save {
    /// Write this state to a file.
    ///
    /// - `path` The path we will write to.
    /// - `state` The app state. This will be converted to a `SerializableState`.
    /// - `conn` The audio connection. Its `SynthState` will be serialized.
    /// - `paths_state` The paths state.
    /// - `exporter` The exporter.
    pub fn write(
        path: &PathBuf,
        state: &State,
        conn: &Conn,
        paths_state: &PathsState,
        exporter: &Exporter,
    ) {
        // Convert the state to something that can be serialized.
        let save = Save {
            state: state.clone(),
            synth_state: conn.state.clone(),
            paths_state: paths_state.clone(),
            exporter: exporter.clone(),
        };
        // Try to open the file.
        match OpenOptions::new()
            .write(true)
            .append(false)
            .truncate(true)
            .create(true)
            .open(path)
        {
            Ok(mut file) => {
                let s = match to_string(&save) {
                    Ok(s) => s,
                    Err(error) => panic!("{} {}", WRITE_ERROR, error),
                };
                if let Err(error) = file.write(s.as_bytes()) {
                    panic!("{} {}", WRITE_ERROR, error)
                }
            }
            Err(error) => panic!("{} {}", WRITE_ERROR, error),
        }
    }

    /// Load a file and deserialize.
    pub fn read(path: &PathBuf, state: &mut State, conn: &mut Conn, paths_state: &mut PathsState, exporter: &mut Exporter) {
        match File::open(path) {
            Ok(mut file) => {
                let mut string = String::new();
                match file.read_to_string(&mut string) {
                    Ok(_) => {
                        let q: Result<Save, Error> = from_str(&string);
                        match q {
                            Ok(s) => {
                                // Set the app state.
                                *state = s.state;

                                // Set the paths.
                                *paths_state = s.paths_state;

                                // Set the exporter.
                                *exporter = s.exporter;

                                // Set the synthesizer.
                                // Set the gain.
                                let mut commands = vec![Command::SetGain {
                                    gain: s.synth_state.gain,
                                }];
                                // Load each SoundFont.
                                for program in s.synth_state.programs.iter() {
                                    if !program.1.path.exists() {
                                        continue;
                                    }
                                    let channel = *program.0;
                                    commands.push(Command::LoadSoundFont {
                                        channel,
                                        path: program.1.path.clone(),
                                    });
                                }
                                // Set each program.                            // Load each SoundFont.
                                for program in s.synth_state.programs.iter() {
                                    if !program.1.path.exists() {
                                        continue;
                                    }
                                    let channel = *program.0;
                                    commands.push(Command::SetProgram {
                                        channel,
                                        path: program.1.path.clone(),
                                        bank_index: program.1.bank_index,
                                        preset_index: program.1.preset_index,
                                    });
                                }

                                // Set the synth state.
                                conn.state = s.synth_state;

                                // Send the commands.
                                conn.send(commands);
                            }
                            Err(error) => panic!("{} {}", READ_ERROR, error),
                        }
                    }
                    Err(error) => panic!("{} {}", READ_ERROR, error),
                }
            }
            Err(error) => panic!("{} {}", READ_ERROR, error),
        }
    }
}
