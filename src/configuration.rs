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
    first_color: Color,
    second_color: Color,
    third_color: Color,
    fourth_color: Color,
    fith_color: Color,
    custom_color: Color,
}

impl ColorPalette {
    pub fn first_color(&self) -> Color {
        self.first_color
    }

    pub fn second_color(&self) -> Color {
        self.second_color
    }

    pub fn third_color(&self) -> Color {
        self.third_color
    }

    pub fn fourth_color(&self) -> Color {
        self.fourth_color
    }

    pub fn fith_color(&self) -> Color {
        self.fith_color
    }

    pub fn custom_color(&self) -> Color {
        self.custom_color
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

        *APP_CONFIG.write() = Self::merge(file, command_line)
    }
    fn merge(file: Option<ConfigurationFile>, command_line: CommandLine) -> Self {
        let mut result = Self {
            input_filename: command_line.filename,
            ..Configuration::default()
        };

        // overwrite with all specified values from config file
        if let Some(file) = file {
            if let Some(v) = file.fullscreen {
                result.fullscreen = v;
            }
            if let Some(v) = file.early_exit {
                result.early_exit = v;
            }
            if let Some(v) = file.initial_tool {
                result.initial_tool = v;
            }
            if let Some(v) = file.copy_command {
                result.copy_command = Some(v);
            }
            // output_filename is not yet supported
            if let Some(v) = file.annotation_size_factor {
                result.annotation_size_factor = v;
            }
            if let Some(v) = file.color_palette {
                result.color_palette = v.into();
            }
        }

        // overwrite with all specified values from command line
        if command_line.fullscreen {
            result.fullscreen = command_line.fullscreen;
        }
        if command_line.early_exit {
            result.early_exit = command_line.early_exit;
        }
        if let Some(v) = command_line.initial_tool {
            result.initial_tool = v.into();
        }
        if let Some(v) = command_line.copy_command {
            result.copy_command = Some(v);
        }
        if let Some(v) = command_line.output_filename {
            result.output_filename = Some(v);
        }
        if let Some(v) = command_line.annotation_size_factor {
            result.annotation_size_factor = v;
        }

        // return result
        result
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
            first_color: Color::orange(),
            second_color: Color::red(),
            third_color: Color::green(),
            fourth_color: Color::blue(),
            fith_color: Color::cove(),
            custom_color: Color::pink(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConfigurationFile {
    fullscreen: Option<bool>,
    early_exit: Option<bool>,
    initial_tool: Option<Tools>,
    copy_command: Option<String>,
    annotation_size_factor: Option<f64>,
    color_palette: Option<ColorPaletteFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ColorPaletteFile {
    first_color: HexColor,
    second_color: HexColor,
    third_color: HexColor,
    fourth_color: HexColor,
    fith_color: HexColor,
    custom_color: HexColor,
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

impl From<ColorPaletteFile> for ColorPalette {
    fn from(value: ColorPaletteFile) -> Self {
        Self {
            first_color: value.first_color.into(),
            second_color: value.second_color.into(),
            third_color: value.third_color.into(),
            fourth_color: value.fourth_color.into(),
            fith_color: value.fith_color.into(),
            custom_color: value.custom_color.into(),
        }
    }
}
