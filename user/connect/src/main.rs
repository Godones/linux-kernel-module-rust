use clap::{Parser, Subcommand};
use domain_helper::register_domain;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

pub enum DomainTypeRaw {
    EmptyDeviceDomain = 1,
    LogDomain = 2,
}

#[derive(Subcommand)]
enum Commands {
    /// Register domain file to the kernel
    Register {
        #[arg(short, long, value_name = "NAME")]
        /// The name of the domain file
        name_file: String,
        #[arg(short, long, value_name = "TYPE")]
        /// The type of the domain
        /// [1: EmptyDeviceDomain]
        /// [2: LogDomain]
        type_: u8,
        #[arg(short, long, value_name = "IDENT")]
        /// The identifier of the domain in the kernel
        ///
        /// if not set, the name of the domain file will be used
        register_ident: Option<String>,
    },
    /// Update domain
    Update {
        #[arg(short, long, value_name = "OLD_NAME")]
        /// The name of the old domain
        old_domain_name: String,
        #[arg(short, long, value_name = "NEW_NAME")]
        /// The name of the new domain
        new_domain_name: String,
        #[arg(short, long, value_name = "TYPE")]
        /// The type of the domain
        /// [1: EmptyDeviceDomain]
        /// [2: LogDomain]
        type_: u8,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Register {
            name_file,
            type_,
            register_ident,
        }) => {
            println!(
                "Register Domain: {}, type: {}, ident: {:?}",
                name_file, type_, register_ident
            );
            let register_ident = if let Some(ident) = register_ident {
                ident
            } else {
                name_file.clone()
            };
            register_domain(&name_file, type_, &register_ident);
        }
        Some(Commands::Update {
            old_domain_name,
            new_domain_name,
            type_,
        }) => {
            println!(
                "Update Domain: {}, new name: {}, type: {}",
                old_domain_name, new_domain_name, type_
            );
        }
        None => {
            println!("No command");
        }
    }
}
