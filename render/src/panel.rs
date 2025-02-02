pub(crate) use crate::drawable::*;
pub(crate) use crate::field_params::*;
pub(crate) use crate::ColorKey;
pub(crate) use common::sizes::*;
pub(crate) use common::PanelType;
use common::VERSION;
pub(crate) use ini::Ini;

/// A panel has a rectangular backaground and a title label.
#[derive(Clone)]
pub(crate) struct Panel {
    /// The type of panel.
    panel_type: PanelType,
    /// The panel background.
    pub background: PanelBackground,
    /// The title label for the panel.
    pub title: LabelRectangle,
}

impl Panel {
    pub fn new(
        panel_type: PanelType,
        position: [u32; 2],
        size: [u32; 2],
        renderer: &Renderer,
        text: &Text,
    ) -> Self {
        // Get the title from the panel type.
        let title = match panel_type {
            PanelType::MainMenu => format!("{} v{}", text.get("TITLE_MAIN_MENU"), VERSION),
            PanelType::Music => text.get("TITLE_MUSIC"),
            PanelType::OpenFile => text.get("TITLE_OPEN_FILE"),
            PanelType::PianoRoll => text.get("TITLE_PIANO_ROLL"),
            PanelType::Tracks => text.get("TITLE_TRACKS"),
            PanelType::ExportState => text.get("TITLE_EXPORT_STATE"),
            PanelType::ExportSettings => text.get("TITLE_EXPORT_SETTINGS"),
            PanelType::Quit => text.get("TITLE_QUIT"),
            PanelType::Links => text.get("TITLE_LINKS"),
        };
        let title_position = [position[0] + 2, position[1]];
        let title = LabelRectangle::new(title_position, title);
        Self {
            panel_type,
            title,
            background: PanelBackground::new(position, size, renderer),
        }
    }

    /// Draw an empty panel. The color will be defined by the value of `focus`.
    pub fn update(&self, focus: bool, renderer: &Renderer) {
        let color: ColorKey = if focus {
            ColorKey::FocusDefault
        } else {
            ColorKey::NoFocus
        };
        self.update_ex(&color, renderer);
    }

    /// Draw an empty panel. The border and title text will be an explicitly defined color.
    pub fn update_ex(&self, color: &ColorKey, renderer: &Renderer) {
        renderer.rectangle_pixel(
            self.background.background.position,
            self.background.background.size,
            &ColorKey::Background,
        );
        renderer.rectangle_lines(&self.background.border, color);
        renderer.rectangle(&self.title.rect, &ColorKey::Background);
        renderer.text(&self.title.label, color);
    }

    /// Returns true if this panel has focus.
    pub fn has_focus(&self, state: &State) -> bool {
        self.panel_type == state.panels[state.focus.get()]
    }
}
