pub(crate) use crate::io_command::IOCommand;
pub(crate) use crate::{get_tooltip, get_tooltip_with_values, Snapshot};
pub(crate) use audio::{Command, Conn};
pub(crate) use common::music_panel_field::MusicPanelField;
pub(crate) use common::{Index, State, PathsState, Paths};
pub(crate) use input::{Input, InputEvent};
pub(crate) use text::{Text, TTS};

/// I guess this is how we do function aliases in Rust.
pub(crate) trait Panel {
    /// Apply panel-specific updates to the state.
    ///
    /// - `state` The state of the app..
    /// - `conn` The synthesizer-player connection.
    /// - `input` Input events, key presses, etc.
    /// - `tts` Text-to-speech.
    /// - `text` The text.
    /// - `paths` Static path data. This is used for setting default paths.
    /// - `paths_state` Dynamic path data.
    ///
    /// Returns: An `Snapshot`.
    fn update(
        &mut self,
        state: &mut State,
        conn: &mut Conn,
        input: &Input,
        tts: &mut TTS,
        text: &Text,
        paths: &Paths,
        paths_state: &mut PathsState
    ) -> Option<Snapshot>;
}
