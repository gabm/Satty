use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLine {
    /// Path to input image or '-' to read from stdin
    #[arg(short, long)]
    pub filename: String,

    /// Start Satty in fullscreen mode
    #[arg(long)]
    pub fullscreen: bool,

    /// Filename to use for saving action, omit to disable saving to file
    #[arg(long)]
    pub output_filename: Option<String>,

    /// Exit directly after copy/save action
    #[arg(long)]
    pub early_exit: bool,

    /// Select the tool on startup
    #[arg(long, value_name = "TOOL", visible_alias = "init-tool")]
    pub initial_tool: Option<Tools>,

    /// Configure the command to be called on copy, for example `wl-copy`
    #[arg(long)]
    pub copy_command: Option<String>,

    /// Increase or decrease the size of the annotations
    #[arg(long)]
    pub annotation_size_factor: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Tools {
    #[default]
    Pointer,
    Crop,
    Line,
    Arrow,
    Rectangle,
    Text,
    Marker,
    Blur,
    Brush,
}

impl std::fmt::Display for Tools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Tools::*;
        let s = match self {
            Pointer => "pointer",
            Crop => "crop",
            Line => "line",
            Arrow => "arrow",
            Rectangle => "rectangle",
            Text => "text",
            Marker => "marker",
            Blur => "blur",
            Brush => "brush",
        };
        f.write_str(s)
    }
}
