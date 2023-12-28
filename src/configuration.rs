use std::{
    fs,
    io::{self, Write},
};

use clap::Parser;
use hex_color::HexColor;
use relm4::SharedState;
use serde_derive::Deserialize;
use thiserror::Error;
use xdg::{BaseDirectories, BaseDirectoriesError};

use crate::{command_line::CommandLine, style::Color, tools::Tools};

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
    initial_tool: Tools,
    copy_command: Option<String>,
    annotation_size_factor: f64,
    color_palette: ColorPalette,
}

pub struct ColorPalette {
    first: Color,
    second: Color,
    third: Color,
    fourth: Color,
    fifth: Color,
    custom: Color,
}

impl ColorPalette {
    pub fn first(&self) -> Color {
        self.first
    }

    pub fn second(&self) -> Color {
        self.second
    }

    pub fn third(&self) -> Color {
        self.third
    }

    pub fn fourth(&self) -> Color {
        self.fourth
    }

    pub fn fifth(&self) -> Color {
        self.fifth
    }

    pub fn custom(&self) -> Color {
        self.custom
    }

    fn merge(&mut self, file_palette: ColorPaletteFile) {
        if let Some(v) = file_palette.first {
            self.first = v.into();
        }
        if let Some(v) = file_palette.second {
            self.second = v.into();
        }
        if let Some(v) = file_palette.third {
            self.third = v.into();
        }
        if let Some(v) = file_palette.fourth {
            self.fourth = v.into();
        }
        if let Some(v) = file_palette.fifth {
            self.fifth = v.into();
        }
        if let Some(v) = file_palette.custom {
            self.custom = v.into();
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
        let file = match ConfigurationFile::try_read() {
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
    fn merge_general(&mut self, general: ConfiguationFileGeneral) {
        if let Some(v) = general.fullscreen {
            self.fullscreen = v;
        }
        if let Some(v) = general.early_exit {
            self.early_exit = v;
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
        }

        // overwrite with all specified values from command line
        if command_line.fullscreen {
            self.fullscreen = command_line.fullscreen;
        }
        if command_line.early_exit {
            self.early_exit = command_line.early_exit;
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
    }

    pub fn early_exit(&self) -> bool {
        self.early_exit
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

    pub fn annotation_size_factor(&self) -> f64 {
        self.annotation_size_factor
    }

    pub fn color_palette(&self) -> &ColorPalette {
        &self.color_palette
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            input_filename: String::new(),
            output_filename: None,
            fullscreen: false,
            early_exit: false,
            initial_tool: Tools::Pointer,
            copy_command: None,
            annotation_size_factor: 1.0f64,
            color_palette: ColorPalette::default(),
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            first: Color::orange(),
            second: Color::red(),
            third: Color::green(),
            fourth: Color::blue(),
            fifth: Color::cove(),
            custom: Color::pink(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConfigurationFile {
    general: Option<ConfiguationFileGeneral>,
    color_palette: Option<ColorPaletteFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConfiguationFileGeneral {
    fullscreen: Option<bool>,
    early_exit: Option<bool>,
    initial_tool: Option<Tools>,
    copy_command: Option<String>,
    annotation_size_factor: Option<f64>,
    output_filename: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ColorPaletteFile {
    first: Option<HexColor>,
    second: Option<HexColor>,
    third: Option<HexColor>,
    fourth: Option<HexColor>,
    fifth: Option<HexColor>,
    custom: Option<HexColor>,
}

impl ConfigurationFile {
    fn try_read() -> Result<Option<ConfigurationFile>, ConfigurationFileError> {
        let dirs = BaseDirectories::with_prefix("satty")?;
        let config_file_path = dirs.get_config_file("config.toml");
        if !config_file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(config_file_path)?;
        Ok(Some(toml::from_str::<ConfigurationFile>(&content)?))
    }
}
