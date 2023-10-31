use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLine {
    /// Name of the person to greet
    #[arg(
        short,
        long,
        help = "Filename to read from, use '-' to read from stdin"
    )]
    pub filename: String,

    #[arg(long, help = "whether to use fullscreen")]
    pub fullscreen: bool,

    #[arg(long, help = "Which filename to use for saving action")]
    pub output_filename: Option<String>,

    #[arg(long, help = "Exit after copy/save")]
    pub early_exit: bool,
}

impl CommandLine {
    pub fn do_parse() -> Self {
        Self::parse()
    }
}
