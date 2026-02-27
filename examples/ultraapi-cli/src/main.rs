//! UltraAPI CLI - Command line interface for UltraAPI applications
//!
//! Usage:
//!   ultraapi run <app_module> --host <host> --port <port>  - Run an UltraAPI application
//!   ultraapi dev <app_module> --host <host> --port <port>  - Run in development mode

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::process::Command;

#[derive(Parser)]
#[command(name = "ultraapi")]
#[command(version = "0.1.0")]
#[command(about = "UltraAPI CLI - Run and develop UltraAPI applications", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an UltraAPI application
    Run {
        /// The application module to run (example name or path)
        app_module: String,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "3000")]
        port: u16,
    },
    /// Run in development mode (with auto-reload - MVP: same as run)
    Dev {
        /// The application module to run (example name or path)
        app_module: String,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "3000")]
        port: u16,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        env::set_var("RUST_LOG", "debug");
    }

    match cli.command {
        Commands::Run {
            app_module,
            host,
            port,
        } => run_app(&app_module, &host, port, false),
        Commands::Dev {
            app_module,
            host,
            port,
        } => run_app(&app_module, &host, port, true),
    }
}

fn run_app(app_module: &str, host: &str, port: u16, dev_mode: bool) -> Result<()> {
    let addr = format!("{}:{}", host, port);

    if dev_mode {
        println!("🚀 Starting UltraAPI in development mode: http://{}", addr);
        println!("   App module: {}", app_module);
        println!("   Note: Auto-reload is not implemented in MVP");
    } else {
        println!("🚀 Starting UltraAPI: http://{}", addr);
        println!("   App module: {}", app_module);
    }

    // Set environment variables for host and port override
    env::set_var("ULTRAAPI_HOST", host);
    env::set_var("ULTRAAPI_PORT", port.to_string());

    // Try to run as a cargo example first
    let cargo_result = run_as_cargo_example(app_module, &addr);

    match cargo_result {
        Ok(_) => Ok(()),
        Err(_) => {
            // Fallback: try to run as a binary in the current directory
            run_as_binary(app_module, &addr)
        }
    }
}

fn run_as_cargo_example(example_name: &str, addr: &str) -> Result<()> {
    // Get the current working directory
    let cwd = env::current_dir().context("Failed to get current directory")?;

    // Try to run the example using cargo
    let status = Command::new("cargo")
        .args(["run", "--example", example_name, "--"])
        .arg(addr)
        .current_dir(&cwd)
        .status()
        .context("Failed to execute cargo run")?;

    if !status.success() {
        anyhow::bail!("cargo run failed with status: {}", status);
    }

    Ok(())
}

fn run_as_binary(binary_name: &str, addr: &str) -> Result<()> {
    // Try to run as a binary
    let status = Command::new("cargo")
        .args(["run", "--bin", binary_name, "--"])
        .arg(addr)
        .status()
        .context("Failed to execute cargo run")?;

    if !status.success() {
        anyhow::bail!(
            "Could not find or run '{}' as an example or binary. \
             Make sure the module exists in your Cargo project.",
            binary_name
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_run() {
        let cli = Cli::parse_from(&["ultraapi", "run", "myapp", "--host", "127.0.0.1", "--port", "8080"]);
        match cli.command {
            Commands::Run { app_module, host, port } => {
                assert_eq!(app_module, "myapp");
                assert_eq!(host, "127.0.0.1");
                assert_eq!(port, 8080);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_parse_dev() {
        let cli = Cli::parse_from(&["ultraapi", "dev", "myapp", "--port", "4000"]);
        match cli.command {
            Commands::Dev { app_module, host, port } => {
                assert_eq!(app_module, "myapp");
                assert_eq!(host, "0.0.0.0");
                assert_eq!(port, 4000);
            }
            _ => panic!("Expected Dev command"),
        }
    }

    #[test]
    fn test_cli_defaults() {
        let cli = Cli::parse_from(&["ultraapi", "run", "myapp"]);
        match cli.command {
            Commands::Run { app_module, host, port } => {
                assert_eq!(app_module, "myapp");
                assert_eq!(host, "0.0.0.0");
                assert_eq!(port, 3000);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_verbose() {
        let cli = Cli::parse_from(&["ultraapi", "-v", "run", "myapp"]);
        assert!(cli.verbose);
    }
}
