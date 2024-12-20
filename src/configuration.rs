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
    command_line::CommandLine,
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
    color_palette: ColorPalette,
    default_hide_toolbars: bool,
    font: FontConfiguration,
    primary_highlighter: Highlighters,
    disable_notifications: bool,
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
        if let Some(v) = general.default_hide_toolbars {
            self.default_hide_toolbars = v;
        }
        if let Some(v) = general.primary_highlighter {
            self.primary_highlighter = v;
        }
        if let Some(v) = general.disable_notifications {
            self.disable_notifications = v;
        }
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

    pub fn color_palette(&self) -> &ColorPalette {
        &self.color_palette
    }

    pub fn default_hide_toolbars(&self) -> bool {
        self.default_hide_toolbars
    }

    pub fn primary_highlighter(&self) -> Highlighters {
        self.primary_highlighter
    }
    pub fn disable_notifications(&self) -> bool {
        self.disable_notifications
    }
    pub fn font(&self) -> &FontConfiguration {
        &self.font
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
            color_palette: ColorPalette::default(),
            default_hide_toolbars: false,
            font: FontConfiguration::default(),
            primary_highlighter: Highlighters::Block,
            disable_notifications: false,
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
    output_filename: Option<String>,
    save_after_copy: Option<bool>,
    default_hide_toolbars: Option<bool>,
    primary_highlighter: Option<Highlighters>,
    disable_notifications: Option<bool>,
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
        let dirs = BaseDirectories::with_prefix("satty")?;
        let config_file_path = dirs.get_config_file("config.toml");
        if !config_file_path.exists() {
            return Ok(None);
        }

        Self::try_read_path(config_file_path)
    }

    fn try_read_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        let content = fs::read_to_string(path)?;
        Ok(Some(toml::from_str::<ConfigurationFile>(&content)?))
    }
}
