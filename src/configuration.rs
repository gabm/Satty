use std::{
    fs,
    io::{self, Write},
};

use clap::Parser;
use serde_derive::Deserialize;
use thiserror::Error;
use xdg::{BaseDirectories, BaseDirectoriesError};

use crate::{command_line::CommandLine, tools::Tools};

#[derive(Error, Debug)]
enum ConfigurationFileError {
    #[error("XDG context error")]
    Xdg(#[from] BaseDirectoriesError),

    #[error("Error reading file")]
    ReadFile(#[from] io::Error),

    #[error("Decoding toml failed")]
    TomlDecoding(#[from] toml::de::Error),
}

pub struct Configuration {
    pub input_filename: String,
    pub output_filename: Option<String>,
    pub fullscreen: bool,
    pub early_exit: bool,
    pub initial_tool: Tools,
    pub copy_command: Option<String>,
}

impl Configuration {
    pub fn load() -> Self {
        // parse commandline options and exit if error
        let command_line = match CommandLine::try_parse() {
            Ok(cmd) => cmd,
            Err(e) => e.exit(),
        };

        // read configuration file and exit on error
        let file = match ConfigurationFile::try_read() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading config file {}", e);

                // swallow broken pipes
                let _ = std::io::stdout().lock().flush();
                let _ = std::io::stderr().lock().flush();

                // exit
                std::process::exit(3);
            }
        };

        Self::merge(file, command_line)
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
        }

        // overwrite with all specified values from command line
        if command_line.fullscreen {
            result.fullscreen = command_line.fullscreen;
        }
        if command_line.early_exit {
            result.early_exit = command_line.early_exit;
        }
        if let Some(v) = command_line.init_tool {
            result.initial_tool = v.into();
        }
        if let Some(v) = command_line.copy_command {
            result.copy_command = Some(v);
        }
        if let Some(v) = command_line.output_filename {
            result.output_filename = Some(v);
        }

        // return result
        result
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
        }
    }
}

#[derive(Deserialize)]
struct ConfigurationFile {
    fullscreen: Option<bool>,
    early_exit: Option<bool>,
    initial_tool: Option<Tools>,
    copy_command: Option<String>,
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
