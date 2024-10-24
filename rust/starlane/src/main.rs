#![allow(warnings)]
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate starlane_macros;

pub static VERSION: Lazy<semver::Version> =
    Lazy::new(|| semver::Version::from_str(env!("CARGO_PKG_VERSION").trim()).unwrap());

pub mod err;
pub mod properties;
pub mod template;

pub mod env;

pub mod platform;

#[cfg(test)]
pub mod test;

//#[cfg(feature="space")]
//pub extern crate starlane_space as starlane;
#[cfg(feature = "space")]
pub mod space {
    pub use starlane_space::space::*;
}

#[cfg(feature = "service")]
pub mod service;

#[cfg(feature = "hyperspace")]
pub mod hyperspace;

#[cfg(feature = "hyperlane")]
pub mod hyperlane;
pub mod registry;

pub mod executor;
pub mod host;

#[cfg(feature = "cli")]
pub mod cli;

pub mod driver;

#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
pub use server::*;

use crate::cli::{Cli, Commands};
use crate::platform::Platform;
use anyhow::anyhow;
use clap::Parser;
use once_cell::sync::Lazy;
use starlane::space::loc::ToBaseKind;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::process;
use std::str::FromStr;
use std::time::Duration;
use tokio::fs;
use tokio::fs::DirEntry;
use tokio::runtime::Builder;
use zip::write::FileOptions;
use crate::env::STARLANE_HOME;

#[cfg(feature = "server")]
async fn config() -> StarlaneConfig {

    let file = format!("{}/config.yaml", STARLANE_HOME.to_string());
    let config = match fs::try_exists(file.clone()).await {
        Ok(true) => {
            match fs::read_to_string(file.clone()).await {
                Ok(config) => {
                    match serde_yaml::from_str(&config).map_err(|e| anyhow!(e)) {
                        Ok(config) => config,
                        Err(err) => {
                            println!("starlane config file '{}' failed to parse: '{}'", file, err.to_string());
                            Default::default()
                        }
                    }
                }
                Err(err) => {
                    println!("starlane config file '{}' error when attempting to read to string: '{}'", file, err.to_string());
                    Default::default()
                }
            }
        }
        Ok(false) => {
           Default::default()
        }
        Err(err) => {
            println!("starlane encountered problem when attempting to load config file: '{}' with error: '{}'", file, err.to_string());
            Default::default()
        }
    };
    config
}



pub fn init() {
    #[cfg(feature = "cli")]
    {
        use rustls::crypto::aws_lc_rs::default_provider;
        default_provider()
            .install_default()
            .expect("crypto provider could not be installed");
    }
}

#[cfg(feature = "cli")]
pub fn main() -> Result<(), anyhow::Error> {
    init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Machine => machine(),
        Commands::Term(args) => {
            let runtime = Builder::new_multi_thread().enable_all().build()?;

            match runtime.block_on(async move { cli::term(args).await }) {
                Ok(_) => Ok(()),
                Err(err) => {
                    println!("err! {}", err.to_string());
                    Err(err.into())
                }
            }
        }
        Commands::Version => {
            println!("{}", VERSION.to_string());
            Ok(())
        }
    }
}

#[cfg(not(feature = "server"))]
fn machine() -> Result<(), anyhow::Error> {
    println!("'' feature is not enabled in this starlane installation");
    Err(anyhow!(
        "'machine' feature is not enabled in this starlane installation"
    ))
}

#[cfg(feature = "server")]
fn machine() -> Result<(), anyhow::Error> {
    ctrlc::set_handler(move || {
        std::process::exit(1);
    });


    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let config = config().await;
        let starlane = Starlane::new(config.registry).await.unwrap();
        let machine_api = starlane.machine();

        let api = tokio::time::timeout(Duration::from_secs(30), machine_api)
            .await
            .unwrap()
            .unwrap();
        // this is a dirty hack which is good enough for a 0.3.0 release...
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
    Ok(())
}

/*
#[no_mangle]
pub extern "C" fn starlane_uuid() -> loc::Uuid {
loc::Uuid::from(uuid::Uuid::new_v4()).unwrap()
}

#[no_mangle]
pub extern "C" fn starlane_timestamp() -> Timestamp {
Timestamp { millis: Utc::now().timestamp_millis() }
}

*/
/*
#[cfg(feature = "cli")]
async fn cli() -> Result<(), SpaceErr> {
    let home_dir: String = match dirs::home_dir() {
        None => ".".to_string(),
        Some(dir) => dir.display().to_string(),
    };
    let matches = ClapCommand::new("cosmic-cli")
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .takes_value(true)
                .value_name("host")
                .required(false)
                .default_value("localhost"),
        )
        .arg(
            Arg::new("certs")
                .short('c')
                .long("certs")
                .takes_value(true)
                .value_name("certs")
                .required(false)
                .default_value(format!("{}/.old/localhost/certs", home_dir).as_str()),
        )
        .subcommand(ClapCommand::new("script"))
        .allow_external_subcommands(true)
        .get_matches();

    let host = matches.get_one::<String>("host").unwrap().clone();
    let certs = matches.get_one::<String>("certs").unwrap().clone();
    let session = Session::new(host, certs).await?;

    if matches.subcommand_name().is_some() {
        session.command(matches.subcommand_name().unwrap()).await
    } else {
        loop {
            let line: String = text_io::try_read!("{};").map_err(|e| SpaceErr::new(500, "err"))?;

            let line_str = line.trim();

            if "exit" == line_str {
                return Ok(());
            }
            println!("> {}", line_str);
            session.command(line.as_str()).await?;
        }
        Ok(())
    }
}

 */

pub fn zip_dir<T>(
    it: impl Iterator<Item = DirEntry>,
    prefix: &str,
    writer: T,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<T>
where
    T: Write + Seek,
{
    let mut zip = zip::ZipWriter::new(writer);
    let options = FileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(prefix)).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            zip.start_file(name.to_str().unwrap(), options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            zip.add_directory(name.to_str().unwrap(), options)?;
        }
    }
    let result = zip.finish()?;
    Result::Ok(result)
}
