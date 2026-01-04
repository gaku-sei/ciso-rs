use std::fs::File;
use std::io;
use std::process;

use ciso_rs::check_ciso;
use ciso_rs::compress_ciso;
use ciso_rs::decompress_ciso;

use crate::args::{Args, Mode};

mod args;

fn main() -> io::Result<()> {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };

    match args.mode {
        Mode::Compress { level } => {
            println!(
                "Compress {} → {} (level {})",
                args.input, args.output, level
            );

            let input = File::open(&args.input)?;
            let output = File::create(&args.output)?;

            compress_ciso(input, output, level)?;
        }
        Mode::Decompress => {
            println!("Decompress {} → {}", args.input, args.output);

            let input = File::open(&args.input)?;
            let output = File::create(&args.output)?;

            decompress_ciso(input, output)?;
        }
        Mode::Check { full } => {
            println!("Check {}", args.input);

            let input = File::open(&args.input)?;

            check_ciso(input, full)?;
        }
    }

    Ok(())
}
