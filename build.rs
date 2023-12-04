#[allow(dead_code)]
#[path = "src/command_line.rs"]
mod command_line;

use std::fs;
use std::io;

use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use clap_complete_fig::Fig;
use clap_complete_nushell::Nushell;

fn main() -> Result<(), io::Error> {
    let cmd = &mut command_line::CommandLine::command();
    let bin = "satty";
    let out = "completions";

    fs::create_dir_all(out)?;
    generate_to(Shell::Bash, cmd, bin, out)?;
    generate_to(Shell::Fish, cmd, bin, out)?;
    generate_to(Shell::Zsh, cmd, bin, out)?;
    generate_to(Shell::Elvish, cmd, bin, out)?;
    generate_to(Nushell, cmd, bin, out)?;
    generate_to(Fig, cmd, bin, out)?;

    Ok(())
}
