use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nlang")]
#[command(about = "A new programming language with Python-like syntax compiled to machine code using LLVM")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Compile an Nlang file to machine code
    #[command(alias = "c")]
    Compile {
        /// Input file to compile
        input: PathBuf,
        
        /// Output file name (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Run an Nlang file directly
    #[command(alias = "r")]
    Run {
        /// Input file to run
        input: PathBuf,
    },
    
    /// Generate LLVM IR from an Nlang file
    #[command(alias = "ir")]
    GenerateIr {
        /// Input file to generate IR from
        input: PathBuf,
        
        /// Output IR file name (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Generate C code from an Nlang file
    #[command(alias = "c-gen")]
    GenerateC {
        /// Input file to generate C code from
        input: PathBuf,
        
        /// Output C file name (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, output } => {
            nlang::cli::compile(input, output)?;
        }
        Commands::Run { input } => {
            nlang::cli::run(input)?;
        }
        Commands::GenerateIr { input, output } => {
            nlang::cli::generate_ir(input, output)?;
        }
        Commands::GenerateC { input, output } => {
            nlang::cli::generate_c(input, output)?;
        }
    }

    Ok(())
}
