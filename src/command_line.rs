use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLine {
    /// Path to the config file. Otherwise will be read from XDG_CONFIG_DIR/satty/config.toml
    #[arg(short, long)]
    pub config: Option<String>,

    /// Path to input image or '-' to read from stdin
    #[arg(short, long)]
    pub filename: String,

    /// Start Satty in fullscreen mode
    #[arg(long)]
    pub fullscreen: bool,

    /// Filename to use for saving action. Omit to disable saving to file. Might contain format
    /// specifiers: <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>.
    #[arg(short, long)]
    pub output_filename: Option<String>,

    /// Exit directly after copy/save action
    #[arg(long)]
    pub early_exit: bool,

    /// Draw corners of rectangles round if the value is greater than 0
    /// (Defaults to 12) (0 disables rounded corners)
    #[arg(long)]
    pub corner_roundness: Option<f32>,

    /// Select the tool on startup
    #[arg(long, value_name = "TOOL", visible_alias = "init-tool")]
    pub initial_tool: Option<Tools>,

    /// Configure the command to be called on copy, for example `wl-copy`
    #[arg(long)]
    pub copy_command: Option<String>,

    /// Increase or decrease the size of the annotations
    #[arg(long)]
    pub annotation_size_factor: Option<f32>,

    /// After copying the screenshot, save it to a file as well
    #[arg(long)]
    pub save_after_copy: bool,

    /// Hide toolbars by default
    #[arg(short, long)]
    pub default_hide_toolbars: bool,

    /// Font family to use for text annotations
    #[arg(long)]
    pub font_family: Option<String>,

    /// Font style to use for text annotations
    #[arg(long)]
    pub font_style: Option<String>,

    /// The primary highlighter to use, secondary is accessible with CTRL.
    #[arg(long)]
    pub primary_highlighter: Option<Highlighters>,

    /// Disable notifications
    #[arg(long)]
    pub disable_notifications: bool,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Tools {
    #[default]
    Pointer,
    Crop,
    Line,
    Arrow,
    Rectangle,
    Ellipse,
    Text,
    Marker,
    Blur,
    Highlight,
    Brush,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Highlighters {
    #[default]
    Block,
    Freehand,
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
            Ellipse => "ellipse",
            Text => "text",
            Marker => "marker",
            Blur => "blur",
            Highlight => "highlight",
            Brush => "brush",
        };
        f.write_str(s)
    }
}
