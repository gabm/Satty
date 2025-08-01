use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use clap::Parser;
use hex_color::HexColor;
use relm4::SharedState;
use serde_derive::Deserialize;
use thiserror::Error;
use xdg::{BaseDirectories, BaseDirectoriesError};

use crate::{
    command_line::{Action as CommandLineAction, CommandLine},
    style::Color,
    tools::{Highlighters, Tools},
};

pub static APP_CONFIG: SharedState<Configuration> = SharedState::new();

#[derive(Error, Debug)]
enum ConfigurationFileError {
    #[error("XDG context error: {0}")]
    Xdg(#[from] BaseDirectoriesError),

    #[error("Error reading file: {0}")]
    ReadFile(#[from] io::Error),

    #[error("Decoding toml failed: {0}")]
    TomlDecoding(#[from] toml::de::Error),
}

pub struct Configuration {
    input_filename: String,
    output_filename: Option<String>,
    fullscreen: bool,
    early_exit: bool,
    corner_roundness: f32,
    initial_tool: Tools,
    copy_command: Option<String>,
    annotation_size_factor: f32,
    save_after_copy: bool,
    actions_on_enter: Vec<Action>,
    actions_on_escape: Vec<Action>,
    actions_on_right_click: Vec<Action>,
    color_palette: ColorPalette,
    default_hide_toolbars: bool,
    focus_toggles_toolbars: bool,
    default_fill_shapes: bool,
    font: FontConfiguration,
    primary_highlighter: Highlighters,
    disable_notifications: bool,
    profile_startup: bool,
    no_window_decoration: bool,
    brush_smooth_history_size: usize,
}

#[derive(Default)]
pub struct FontConfiguration {
    family: Option<String>,
    style: Option<String>,
}

impl FontConfiguration {
    pub fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }
    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }
    fn merge(&mut self, file_font: FontFile) {
        if let Some(v) = file_font.family {
            self.family = Some(v);
        }
        if let Some(v) = file_font.style {
            self.style = Some(v);
        }
    }
}

pub struct ColorPalette {
    palette: Vec<Color>,
    custom: Vec<Color>,
}

impl ColorPalette {
    pub fn palette(&self) -> &[Color] {
        &self.palette
    }

    pub fn custom(&self) -> &[Color] {
        &self.custom
    }

    fn merge(&mut self, file_palette: ColorPaletteFile) {
        if let Some(v) = file_palette.palette {
            self.palette = v.into_iter().map(Color::from).collect();
        }
        if let Some(v) = file_palette.custom {
            self.custom = v.into_iter().map(Color::from).collect();
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    SaveToClipboard,
    SaveToFile,
    Exit,
}

impl From<CommandLineAction> for Action {
    fn from(action: CommandLineAction) -> Self {
        match action {
            CommandLineAction::SaveToClipboard => Self::SaveToClipboard,
            CommandLineAction::SaveToFile => Self::SaveToFile,
            CommandLineAction::Exit => Self::Exit,
        }
    }
}

impl Configuration {
    pub fn load() {
        // parse commandline options and exit if error
        let command_line = match CommandLine::try_parse() {
            Ok(cmd) => cmd,
            Err(e) => e.exit(),
        };

        // read configuration file and exit on error
        let file = match ConfigurationFile::try_read(&command_line.config) {
            Ok(c) => c,
            Err(ConfigurationFileError::ReadFile(e)) if e.kind() == io::ErrorKind::NotFound => {
                eprintln!("config file not found");
                None
            }
            Err(e) => {
                eprintln!("Error reading config file: {e}");

                // swallow broken pipes
                let _ = std::io::stdout().lock().flush();
                let _ = std::io::stderr().lock().flush();

                // exit
                std::process::exit(3);
            }
        };

        APP_CONFIG.write().merge(file, command_line);
    }
    fn merge_general(&mut self, general: ConfigurationFileGeneral) {
        if let Some(v) = general.fullscreen {
            self.fullscreen = v;
        }
        if let Some(v) = general.early_exit {
            self.early_exit = v;
        }
        if let Some(v) = general.corner_roundness {
            self.corner_roundness = v;
        }
        if let Some(v) = general.initial_tool {
            self.initial_tool = v;
        }
        if let Some(v) = general.copy_command {
            self.copy_command = Some(v);
        }
        if let Some(v) = general.output_filename {
            self.output_filename = Some(v);
        }
        if let Some(v) = general.annotation_size_factor {
            self.annotation_size_factor = v;
        }
        if let Some(v) = general.save_after_copy {
            self.save_after_copy = v;
        }
        if let Some(v) = general.actions_on_enter {
            self.actions_on_enter = v;
        }
        if let Some(v) = general.actions_on_escape {
            self.actions_on_escape = v;
        }
        if let Some(v) = general.actions_on_right_click {
            self.actions_on_right_click = v;
        }
        if let Some(v) = general.default_hide_toolbars {
            self.default_hide_toolbars = v;
        }
        if let Some(v) = general.focus_toggles_toolbars {
            self.focus_toggles_toolbars = v;
        }
        if let Some(v) = general.default_fill_shapes {
            self.default_fill_shapes = v;
        }
        if let Some(v) = general.primary_highlighter {
            self.primary_highlighter = v;
        }
        if let Some(v) = general.disable_notifications {
            self.disable_notifications = v;
        }
        if let Some(v) = general.no_window_decoration {
            self.no_window_decoration = v;
        }
        if let Some(v) = general.brush_smooth_history_size {
            self.brush_smooth_history_size = v;
        }

        // --- deprecated options ---
        if let Some(v) = general.right_click_copy {
            if v && !self
                .actions_on_right_click
                .contains(&Action::SaveToClipboard)
            {
                self.actions_on_right_click
                    .insert(0, Action::SaveToClipboard);
            }
        }
        if let Some(v) = general.action_on_enter {
            self.actions_on_enter.insert(0, v);
        }
        // ---
    }
    fn merge(&mut self, file: Option<ConfigurationFile>, command_line: CommandLine) {
        // input_filename is required and needs to be overwritten
        self.input_filename = command_line.filename;

        // overwrite with all specified values from config file
        if let Some(file) = file {
            if let Some(general) = file.general {
                self.merge_general(general);
            }
            if let Some(v) = file.color_palette {
                self.color_palette.merge(v);
            }
            if let Some(v) = file.font {
                self.font.merge(v);
            }
        }

        // overwrite with all specified values from command line
        if command_line.fullscreen {
            self.fullscreen = command_line.fullscreen;
        }
        if command_line.early_exit {
            self.early_exit = command_line.early_exit;
        }
        if let Some(v) = command_line.corner_roundness {
            self.corner_roundness = v;
        }
        if command_line.default_hide_toolbars {
            self.default_hide_toolbars = command_line.default_hide_toolbars;
        }
        if command_line.focus_toggles_toolbars {
            self.focus_toggles_toolbars = command_line.focus_toggles_toolbars
        }
        if command_line.default_fill_shapes {
            self.default_fill_shapes = command_line.default_fill_shapes;
        }
        if let Some(v) = command_line.initial_tool {
            self.initial_tool = v.into();
        }
        if let Some(v) = command_line.copy_command {
            self.copy_command = Some(v);
        }
        if let Some(v) = command_line.output_filename {
            self.output_filename = Some(v);
        }
        if let Some(v) = command_line.annotation_size_factor {
            self.annotation_size_factor = v;
        }
        if command_line.save_after_copy {
            self.save_after_copy = command_line.save_after_copy;
        }
        if let Some(v) = command_line.actions_on_enter {
            self.actions_on_enter = v.iter().cloned().map(Into::into).collect();
        }
        if let Some(v) = command_line.actions_on_escape {
            self.actions_on_escape = v.iter().cloned().map(Into::into).collect();
        }
        if let Some(v) = command_line.actions_on_right_click {
            self.actions_on_right_click = v.iter().cloned().map(Into::into).collect();
        }
        if let Some(v) = command_line.font_family {
            self.font.family = Some(v);
        }
        if let Some(v) = command_line.font_style {
            self.font.style = Some(v);
        }
        if let Some(v) = command_line.primary_highlighter {
            self.primary_highlighter = v.into();
        }
        if command_line.disable_notifications {
            self.disable_notifications = command_line.disable_notifications;
        }
        if command_line.profile_startup {
            self.profile_startup = command_line.profile_startup;
        }
        if command_line.no_window_decoration {
            self.no_window_decoration = command_line.no_window_decoration;
        }
        if let Some(v) = command_line.brush_smooth_history_size {
            self.brush_smooth_history_size = v;
        }

        // --- deprecated options ---
        if command_line.right_click_copy
            && !self
                .actions_on_right_click
                .contains(&Action::SaveToClipboard)
        {
            self.actions_on_right_click
                .insert(0, Action::SaveToClipboard);
        }
        if let Some(v) = command_line.action_on_enter {
            self.actions_on_enter.insert(0, v.into());
        }
        // ---
    }

    pub fn early_exit(&self) -> bool {
        self.early_exit
    }

    pub fn corner_roundness(&self) -> f32 {
        self.corner_roundness
    }

    pub fn initial_tool(&self) -> Tools {
        self.initial_tool
    }

    pub fn copy_command(&self) -> Option<&String> {
        self.copy_command.as_ref()
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn output_filename(&self) -> Option<&String> {
        self.output_filename.as_ref()
    }

    pub fn input_filename(&self) -> &str {
        self.input_filename.as_ref()
    }

    pub fn annotation_size_factor(&self) -> f32 {
        self.annotation_size_factor
    }

    pub fn save_after_copy(&self) -> bool {
        self.save_after_copy
    }

    pub fn actions_on_enter(&self) -> Vec<Action> {
        self.actions_on_enter.clone()
    }

    pub fn actions_on_escape(&self) -> Vec<Action> {
        self.actions_on_escape.clone()
    }

    pub fn actions_on_right_click(&self) -> Vec<Action> {
        self.actions_on_right_click.clone()
    }

    pub fn color_palette(&self) -> &ColorPalette {
        &self.color_palette
    }

    pub fn default_hide_toolbars(&self) -> bool {
        self.default_hide_toolbars
    }

    pub fn focus_toggles_toolbars(&self) -> bool {
        self.focus_toggles_toolbars
    }

    pub fn default_fill_shapes(&self) -> bool {
        self.default_fill_shapes
    }

    pub fn primary_highlighter(&self) -> Highlighters {
        self.primary_highlighter
    }

    pub fn disable_notifications(&self) -> bool {
        self.disable_notifications
    }

    pub fn profile_startup(&self) -> bool {
        self.profile_startup
    }

    pub fn no_window_decoration(&self) -> bool {
        self.no_window_decoration
    }

    pub fn font(&self) -> &FontConfiguration {
        &self.font
    }

    pub fn brush_smooth_history_size(&self) -> usize {
        self.brush_smooth_history_size
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            input_filename: String::new(),
            output_filename: None,
            fullscreen: false,
            early_exit: false,
            corner_roundness: 12.0,
            initial_tool: Tools::Pointer,
            copy_command: None,
            annotation_size_factor: 1.0,
            save_after_copy: false,
            actions_on_enter: vec![],
            actions_on_escape: vec![Action::Exit],
            actions_on_right_click: vec![],
            color_palette: ColorPalette::default(),
            default_hide_toolbars: false,
            focus_toggles_toolbars: false,
            default_fill_shapes: false,
            font: FontConfiguration::default(),
            primary_highlighter: Highlighters::Block,
            disable_notifications: false,
            profile_startup: false,
            no_window_decoration: false,
            brush_smooth_history_size: 0, // default to 0, no history
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            palette: vec![
                Color::orange(),
                Color::red(),
                Color::green(),
                Color::blue(),
                Color::cove(),
            ],
            custom: vec![],
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConfigurationFile {
    general: Option<ConfigurationFileGeneral>,
    color_palette: Option<ColorPaletteFile>,
    font: Option<FontFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct FontFile {
    family: Option<String>,
    style: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConfigurationFileGeneral {
    fullscreen: Option<bool>,
    early_exit: Option<bool>,
    corner_roundness: Option<f32>,
    initial_tool: Option<Tools>,
    copy_command: Option<String>,
    annotation_size_factor: Option<f32>,
    save_after_copy: Option<bool>,
    output_filename: Option<String>,
    actions_on_enter: Option<Vec<Action>>,
    actions_on_escape: Option<Vec<Action>>,
    actions_on_right_click: Option<Vec<Action>>,
    default_hide_toolbars: Option<bool>,
    focus_toggles_toolbars: Option<bool>,
    default_fill_shapes: Option<bool>,
    primary_highlighter: Option<Highlighters>,
    disable_notifications: Option<bool>,
    no_window_decoration: Option<bool>,
    brush_smooth_history_size: Option<usize>,

    // --- deprecated options ---
    right_click_copy: Option<bool>,
    action_on_enter: Option<Action>,
    // ---
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ColorPaletteFile {
    palette: Option<Vec<HexColor>>,
    custom: Option<Vec<HexColor>>,
}

impl ConfigurationFile {
    fn try_read(
        specified_path: &Option<String>,
    ) -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        match specified_path {
            None => Self::try_read_xdg(),
            Some(p) => Self::try_read_path(p),
        }
    }

    fn try_read_xdg() -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        let dirs = BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"));
        match dirs.get_config_file("config.toml") {
            Some(path) => Self::try_read_path(path),
            None => Ok(None),
        }
    }

    fn try_read_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        let content = fs::read_to_string(path)?;
        Ok(Some(toml::from_str::<ConfigurationFile>(&content)?))
    }
}
