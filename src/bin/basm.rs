use clap::{Parser, ValueEnum};
use std::{fmt::{Display, Formatter, Result as FmtResult}, io::{Read, Result, Write}};

use bfcomp::{util::ascii::check_valid_template, *};
use tracing::*;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// The input file to assemble.
    /// If not specified, the input will be read from stdin.
    pub input: Option<String>,

    /// The output file to write the assembled code to.
    /// If not specified, the output will be written to stdout.
    #[arg(short, long)]
    pub output: Option<String>,

    /// The source to compile
    #[arg(short, long, value_enum, default_value_t = Source::Assembly)]
    pub source: Source,

    /// The backend to target
    #[arg(short, long, value_enum, default_value_t = Backend::Run)]
    pub target: Backend,

    /// Compile in release mode
    #[arg(short, long, default_value_t = true)]
    pub release: bool,

    /// The ASCII art template to apply to output BrainFuck code.
    /// If not specified, no template will be applied.
    /// Pass `--art list` to see the list of available templates.
    #[arg(short, long)]
    pub art: Option<String>,

    /// The comment to use in the ASCII art template.
    /// If not specified, no comment will be used.
    #[arg(short, long)]
    pub comment: Option<String>,
}

fn main() {
    init_logging(); 
    let args = Args::parse();

    if args.art.as_ref().map(|x| x.as_str()) == Some("list") {
        let templates = util::ascii::get_template_names();
        info!("Available ASCII art templates:");
        for template in templates {
            info!("• {}", template);
        }
        std::process::exit(0);
    } else if let Some(template) = args.art.as_ref() {
        if !check_valid_template(template) {
            std::process::exit(1);
        }
    }

    if let Err(e) = build_for_backend(&args) {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn source_from_input_file(args: &Args) -> Result<Source> {
    match &args.input {
        Some(input) => {
            // Return the source based on the file extension
            let new_source = Source::from_path(input)
                .ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file extension, expected .b, .bf, or .asm"))?;

            if !args.source.is_compatible_with(&new_source) {
                warn!("Overriding source to {}", new_source);
                Ok(new_source)
            } else {
                info!("Using provided source language: {}", new_source);
                Ok(args.source)
            }
        },
        None => {
            Ok(args.source)
        }
    }
}

fn backend_from_output_file(args: &Args) -> Result<Backend> {
    match &args.output {
        Some(output) => {
            // Get the file extension
            let new_backend = Backend::from_path(output)
                .ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file extension, expected .b, .bf, or .asm"))?;

            if !args.target.is_compatible_with(&new_backend) {
                warn!("Overriding target to {}", new_backend);
                Ok(new_backend)
            } else {
                info!("Using provided target language: {}", new_backend);
                Ok(args.target)
            }
        },
        None => {
            Ok(args.target)
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "kebab_case")]
pub enum Source {
    #[value(alias("b"), alias("bf"))]
    BrainFuck,
    #[value(alias("asm"), alias("basm"))]
    Assembly,
}

impl Source {
    pub fn is_compatible_with(&self, other: &Source) -> bool {
        if self == other {
            return true;
        }
        match (self, other) {
            (Self::BrainFuck, Self::BrainFuck) => true,
            (Self::Assembly, Self::Assembly) => true,
            _ => false,
        }
    }

    pub fn from_file_extension(extension: &str) -> Option<Self> {
        match extension {
            "b" | "bf" => Some(Source::BrainFuck),
            "basm" | "asm" => Some(Source::Assembly),
            _ => None,
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        Self::from_file_extension(extension)
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::BrainFuck => write!(f, "BrainFuck"),
            Self::Assembly => write!(f, "C"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "kebab_case")]
pub enum Backend {
    #[value(alias("bf"))]
    BrainFuck,
    #[value(alias("c"), alias("c8"))]
    C,
    #[value(alias("c16"))]
    C16Bit,
    #[value(alias("c32"))]
    C32Bit,
    #[value(alias("exe"), alias("exe8"))]
    Exe,
    #[value(alias("exe16"))]
    Exe16Bit,
    #[value(alias("exe32"))]
    Exe32Bit,
    #[value(alias("run"), alias("run8"))]
    Run,
    #[value(alias("run16"))]
    Run16Bit,
    #[value(alias("run32"))]
    Run32Bit,
}

impl Display for Backend {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::BrainFuck => write!(f, "BrainFuck"),
            Self::C => write!(f, "C"),
            Self::C16Bit => write!(f, "C 16-bit"),
            Self::C32Bit => write!(f, "C 32-bit"),
            Self::Exe => write!(f, "Executable"),
            Self::Exe16Bit => write!(f, "16-bit Executable"),
            Self::Exe32Bit => write!(f, "32-bit Executable"),
            Self::Run => write!(f, "Run"),
            Self::Run16Bit => write!(f, "Run 16-bit"),
            Self::Run32Bit => write!(f, "Run 32-bit"),
        }
    }
}

impl Backend {
    pub fn bytes(&self) -> u8 {
        match self {
            Backend::C16Bit | Backend::Exe16Bit | Backend::Run16Bit => 2,
            Backend::C32Bit | Backend::Exe32Bit | Backend::Run32Bit => 4,
            _ => 1,
        }
    }

    pub fn is_compatible_with(&self, other: &Backend) -> bool {
        if self == other {
            return true;
        }
        match (self, other) {
            (
                Backend::C
                | Backend::C16Bit
                | Backend::C32Bit,
                Backend::C
                | Backend::C16Bit
                | Backend::C32Bit,
            ) => true,
            (
                Backend::Exe
                | Backend::Exe16Bit
                | Backend::Exe32Bit,
                Backend::Exe
                | Backend::Exe16Bit
                | Backend::Exe32Bit,
            ) => true,
            (
                Backend::Run
                | Backend::Run16Bit
                | Backend::Run32Bit,
                Backend::Run
                | Backend::Run16Bit
                | Backend::Run32Bit,
            ) => true,
            (Backend::BrainFuck, Backend::BrainFuck) => true,
            _ => false,
        }
    }

    pub fn from_file_extension(extension: &str) -> Option<Self> {
        match extension {
            "b" | "bf" => Some(Backend::BrainFuck),
            "c" => Some(Backend::C),
            "" | "exe" => Some(Backend::Exe),
            _ => None,
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        Self::from_file_extension(extension)
    }

    pub fn to_file_extension(&self) -> &str {
        match self {
            Backend::BrainFuck => "bf",
            Backend::C => "c",
            Backend::Exe => "exe",
            _ => "",
        }
    }
}

pub fn read_input_file(args: &Args) -> Result<String> {
    // std::fs::read_to_string(&args.input)
    match &args.input {
        Some(input) => {
            let result = std::fs::read_to_string(input)?;
            info!("Reading from {}", input);
            Ok(result)
        },
        None => {
            info!("Reading from stdin");
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            Ok(input)
        }
    }
}

pub fn read_source_to_bf(args: &Args) -> Result<String> {
    Ok(match source_from_input_file(args)? {
        Source::BrainFuck => {
            info!("Reading BrainFuck source");
            simplify_bf(read_input_file(args)?)
        },
        Source::Assembly => {
            info!("Reading Assembly source");
            let program = Program::parse(&read_input_file(args)?)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            program.assemble()
        },
    })
}

pub fn write_output_file(args: &Args, output: &[u8]) -> Result<()> {
    match &args.output {
        Some(path) => {
            std::fs::write(path, output)?;
            info!("Successfully wrote output to {path}");
            Ok(())
        },
        None => {
            std::io::stdout().write_all(output)?;
            info!("Successfully wrote output to stdout");
            Ok(())
        },
    }
}

pub fn make_ascii_art(args: &Args, bf: String) -> Result<String> {
    if let Some(template) = &args.art {
        info!("Using ASCII art template: {}", template);
        let comment = args.comment.as_ref().map(|s| s.as_str());
        
        util::ascii::apply_template_from_name_or_file(template, bf, comment)
    } else {
        Ok(bf)
    }
}

pub fn build_for_backend(args: &Args) -> Result<()> {
    let bf = read_source_to_bf(args)?;
    let bytes = args.target.bytes();
    let backend = backend_from_output_file(args)?;
    match backend {
        Backend::C
        | Backend::C16Bit 
        | Backend::C32Bit => {
            write_output_file(args, compile_to_c(bf, bytes).as_bytes())?;
        },

        Backend::Exe
        | Backend::Exe16Bit
        | Backend::Exe32Bit if args.output.is_some() => {
            info!("Creating executable...");
            // First, compile to C
            let c = compile_to_c(bf, bytes);

            // Now, write to a temp file
            let mut temp_file = std::env::temp_dir();
            temp_file.push("bfcomp_temp.c");
            std::fs::write(&temp_file, c)?;
            // Now, compile the temp file to an executable
            if args.release {
                std::process::Command::new("gcc")
                    .arg("-Ofast")
                    .arg("-o")
                    .arg(args.output.as_ref().unwrap())
                    .arg(&temp_file)
                    .status()?;
            } else {
                std::process::Command::new("gcc")
                    .arg("-g")
                    .arg("-o")
                    .arg(args.output.as_ref().unwrap())
                    .arg(&temp_file)
                    .status()?;
            }

            // Now, delete the temp file
            std::fs::remove_file(temp_file)?;

            // Now, we're done
            info!("Successfully compiled to executable {}", args.output.as_ref().unwrap());
        }
        Backend::Exe
        | Backend::Exe16Bit
        | Backend::Exe32Bit => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Output file must be specified when targeting an executable",
            ));
        }

        Backend::Run
        | Backend::Run16Bit
        | Backend::Run32Bit => {
            // First, compile to C
            let c = compile_to_c(bf, bytes);

            // Now, write to a temp file
            let temp_dir = std::env::temp_dir();
            let mut temp_c = temp_dir.clone();
            temp_c.push("bfcomp_temp.c");
            std::fs::write(&temp_c, c)?;

            let mut temp_exe = temp_dir;
            temp_exe.push("bfcomp_temp.exe");

            // Now, compile the temp file to an executable
            if args.release {
                std::process::Command::new("gcc")
                    .arg("-Ofast")
                    .arg("-o")
                    .arg(&temp_exe)
                    .arg(&temp_c)
                    .status()?;
            } else {
                std::process::Command::new("gcc")
                    .arg("-g")
                    .arg("-o")
                    .arg(&temp_exe)
                    .arg(&temp_c)
                    .status()?;
            }

            // Now, run the executable
            info!("Running executable...");
            std::process::Command::new(&temp_exe)
                .status()?;

            // Now, delete the temp file
            std::fs::remove_file(temp_c)?;
            std::fs::remove_file(temp_exe)?;

            // Now, we're done
            info!("Successfully ran code");
        }
        Backend::BrainFuck => {
            write_output_file(args, make_ascii_art(args, simplify_bf(bf))?.as_bytes())?;
        }
    }

    Ok(())
}