use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLine {
    /// Name of the person to greet
    #[arg(
        short,
        long,
        help = "Path to input image or '-' to read from stdin."
    )]
    pub filename: String,

    #[arg(long, help = "Start Satty in fullscreen mode.")]
    pub fullscreen: bool,

    #[arg(long, help = "Filename to use for saving action, omit to disable saving to file.")]
    pub output_filename: Option<String>,

    #[arg(long, help = "Exit directly after copy/save action.")]
    pub early_exit: bool,
}

impl CommandLine {
    pub fn do_parse() -> Self {
        Self::parse()
    }
}
