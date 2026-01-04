use std::env;
use std::path::Path;

#[derive(Debug)]
pub enum Mode {
    Compress { level: u32 },
    Decompress,
    Check { full: bool },
}

#[derive(Debug)]
pub struct Args {
    pub mode: Mode,
    pub input: String,
    pub output: String,
}

impl Args {
    pub fn parse() -> Result<Args, String> {
        let mut args = env::args().skip(1).collect::<Vec<_>>();

        if args.is_empty() {
            return Err(usage());
        }

        let input = args.remove(0);
        let ext = Path::new(&input)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "iso" => Self::parse_compress(input, &args),
            "cso" => Self::parse_decompress(input, &args),
            _ => Err("Input must be .iso or .cso".to_string()),
        }
    }

    fn parse_compress(input: String, args: &[String]) -> Result<Args, String> {
        let mut output = None;
        let mut level = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--fast" => {
                    level = Some(1);
                    i += 1;
                }
                "--optimal" => {
                    level = Some(6);
                    i += 1;
                }
                "--best" => {
                    level = Some(9);
                    i += 1;
                }
                "--level" => {
                    if i + 1 >= args.len() {
                        return Err("--level requires a value".to_string());
                    }
                    let v = args[i + 1]
                        .parse::<u32>()
                        .map_err(|_| "Invalid --level value")?;
                    if !(1..=9).contains(&v) {
                        return Err("--level must be 1..9".to_string());
                    }
                    level = Some(v);
                    i += 2;
                }
                s if s.starts_with("--") => {
                    return Err(format!("Unknown option '{s}'"));
                }
                s => {
                    if output.is_some() {
                        return Err("Too many positional arguments".to_string());
                    }
                    output = Some(s.to_string());
                    i += 1;
                }
            }
        }

        let output = output.unwrap_or_else(|| default_out(&input, "cso"));
        let level = level.unwrap_or(6);

        Ok(Args {
            mode: Mode::Compress { level },
            input,
            output,
        })
    }

    fn parse_decompress(input: String, args: &[String]) -> Result<Args, String> {
        let mut check = false;
        let mut full = false;

        for arg in args {
            match arg.as_str() {
                "--check" => check = true,
                "--full" => full = true,
                _ => return Err(format!("Unknown option '{arg}'")),
            }
        }

        if full && !check {
            return Err("--full can only be used with --check".to_string());
        }

        if check {
            if args.len() > 2 {
                return Err("Too many arguments for --check".to_string());
            }

            return Ok(Args {
                mode: Mode::Check { full },
                input,
                output: String::new(),
            });
        }

        if !args.is_empty() {
            return Err("Options are not allowed when decompressing".to_string());
        }

        let output = default_out(&input, "iso");

        Ok(Args {
            mode: Mode::Decompress,
            input,
            output,
        })
    }
}

fn default_out(input: &str, ext: &str) -> String {
    let path = Path::new(input);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or(Path::new(""));
    parent
        .join(format!("{stem}.{ext}"))
        .to_string_lossy()
        .into_owned()
}

fn usage() -> String {
    r"Usage:
  ciso <input.iso> [output.cso] [--level 1..9 | --fast | --optimal | --best]
  ciso <input.cso> [output.iso]
  ciso <input.cso> --check [--full]

Rules:
  .iso → compress
  .cso → decompress

Defaults:
  compress level = 6 (--optimal)
"
    .to_string()
}
