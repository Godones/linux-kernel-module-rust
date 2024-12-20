mod subcommand;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generates a new domain project
    New {
        /// The name of the domain project
        #[arg(short, long, value_name = "NAME")]
        name: String,
    },
    Build {
        /// The name of the domain project
        #[arg(short, long, value_name = "NAME")]
        name: String,
        /// The log level, default is INFO
        #[arg(short, long, value_name = "LOG", default_value = "")]
        log: String,
        /// The target architecture, default is riscv64
        #[arg(short, long, value_name = "ARCH")]
        arch: Option<String>,
    },
    BuildAll {
        /// The log level, default is INFO
        #[arg(short, long, value_name = "LOG", default_value = "")]
        log: String,
        /// The target architecture, default is riscv64
        #[arg(short, long, value_name = "ARCH")]
        arch: Option<String>,
    },
    Move {
        /// The name of the domain project
        #[arg(short, long, value_name = "NAME", default_value = "")]
        name: String,
    },
    Clean {
        /// The name of the domain project
        #[arg(short, long, value_name = "NAME", default_value = "")]
        name: String,
    },
    Fmt {
        /// The name of the domain project
        #[arg(short, long, value_name = "NAME", default_value = "")]
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::New { name }) => {
            println!("Creating new domain project: {}", name);
            subcommand::new::create_domain(name);
        }
        Some(Commands::BuildAll { log, arch }) => {
            println!("Building all domain projects, LOG: {log}, ARCH: {:?}", arch);
            subcommand::build::build_all(log.to_string(), arch.clone());
        }
        Some(Commands::Build { name, log, arch }) => {
            println!(
                "Building domain project: {}, LOG: {}, ARCH: {:?}",
                name, log, arch
            );
            subcommand::build::build_single(name, log, arch.clone());
        }
        Some(Commands::Move { name }) => {
            println!("Moving domain project: {}", name);
            subcommand::r#move::remove_to_space();
        }
        Some(Commands::Clean { name }) => {
            println!("Cleaning domain project: {}", name);
            subcommand::clean::clean_domain(name.to_string());
        }
        Some(Commands::Fmt { name }) => {
            println!("Formatting domain project: {}", name);
            subcommand::fmt::fmt_domain(name.to_string());
        }
        None => {}
    }
}
