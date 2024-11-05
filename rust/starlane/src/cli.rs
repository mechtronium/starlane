<<<<<<<< HEAD:rust/cosmic/cosmic-cli/src/main.rs
    #![allow(warnings)]

pub mod cli;
pub mod err;
pub mod model;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate lazy_static;

use crate::err::CliErr;
use crate::model::{AccessTokenResp, LoginResp};
use clap::arg;
use clap::command;
use clap::{App, Arg, Args, Command as ClapCommand, Parser, Subcommand};
use cosmic_hyperlane::test_util::SingleInterchangePlatform;
use cosmic_hyperlane::HyperwayEndpointFactory;
use cosmic_hyperlane_tcp::HyperlaneTcpClient;
use cosmic_hyperspace::driver::control::{ControlCliSession, ControlClient};
use cosmic_nom::{new_span, Span};
use cosmic_space::command::{CmdTransfer, Command, RawCommand};
use cosmic_space::err::SpaceErr;
use cosmic_space::hyper::{InterchangeKind, Knock};
use cosmic_space::loc::ToSurface;
use cosmic_space::log::RootLogger;
use cosmic_space::parse::error::result;
use cosmic_space::parse::{command_line, upload_blocks};
use cosmic_space::point::Point;
use cosmic_space::substance::Substance;
use cosmic_space::util::{log, ToResolved};
use cosmic_space::wave::core::ReflectedCore;
use nom::bytes::complete::{is_not, tag};
use nom::multi::{separated_list0, separated_list1};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
========
use crate::driver::control::{ControlCliSession, ControlClient};
use crate::hyperlane::tcp::HyperlaneTcpClient;
use crate::hyperlane::HyperwayEndpointFactory;
use clap::clap_derive::{Args, Subcommand};
use clap::Parser;
use starlane_space::space::parse::util::new_span;
use starlane::space::command::{CmdTransfer, RawCommand};
use starlane::space::err::SpaceErr;
use starlane::space::hyper::Knock;
use starlane::space::parse::{upload_blocks, SkewerCase};
use starlane::space::point::Point;
use starlane::space::substance::Substance;
use starlane::space::wave::core::ReflectedCore;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;
>>>>>>>> release/0.3.20:rust/starlane/src/cli.rs
use std::str::FromStr;
use std::time::Duration;
<<<<<<<< HEAD:rust/cosmic/cosmic-cli/src/main.rs
use std::{
    fs, io,
    io::{Cursor, Read, Seek, Write},
    path::Path,
};
use serde_json::json;
use walkdir::{DirEntry, WalkDir};
use zip::{result::ZipError, write::FileOptions};
use crate::cli::CliConfig;
========
use strum_macros::EnumString;
use tokio::io::AsyncWriteExt;
use walkdir::{DirEntry, WalkDir};
use zip::write::FileOptions;
use starlane::space::parse::util::result;
use starlane_primitive_macros::logger;
use crate::env::STARLANE_HOME;
>>>>>>>> release/0.3.20:rust/starlane/src/cli.rs

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(
    short,
            long,
            default_value_t = true
    )]
    pub logs: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand, EnumString, strum_macros::Display)]
#[command(version, about, long_about = None)]
pub enum Commands {
    Install{
        #[arg(long,short)]
        edit: bool,
        #[arg(long,short)]
        nuke: bool
    },
    Run,
    Term(TermArgs),
    Version,
    Splash,
    Scorch,
    Nuke{
        #[arg(long)]
        all: bool
    },
    Context(ContextArgs)
}

#[derive(Debug,Args)]
pub struct ContextArgs{
    #[clap(subcommand)]
    pub command: ContextCmd,
}

impl Default for ContextArgs {
    fn default() -> Self {
        todo!()
    }
}

#[derive(Debug,Subcommand,EnumString, strum_macros::Display)]
pub enum ContextCmd {
    Create{ context_name: String},
    Switch{ context_name: String},
    Default,
    List,
    Which
}

#[derive(Debug, Args)]
pub struct TermArgs {
    #[arg(long)]
    host: Option<String>,

    /// Number of times to greet
    #[arg(long)]
    certs: Option<String>,

    #[arg(long)]
    history_log: Option<String>
}

impl Default for TermArgs {
    fn default() -> Self {

        Self {
            host: None,
            certs: None,
            history_log: None
        }
    }
}

pub async fn term(args: TermArgs) -> Result<(), SpaceErr> {
    let history_log = match args.history_log {
        None => format!("{}/history.log", STARLANE_HOME.to_string()).to_string(),
        Some(history) => history.to_string(),
    };
<<<<<<<< HEAD:rust/cosmic/cosmic-cli/src/main.rs
    let matches = ClapCommand::new("cosmic")
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .takes_value(true)
                .value_name("host")
                .required(false)
                .default_value("localhost:4343"),
        )
        .arg(
            Arg::new("certs")
                .short('c')
                .long("certs")
                .takes_value(true)
                .value_name("certs")
                .required(false)
                .default_value(format!("{}/.starlane/localhost/certs", home_dir).as_str()),
        )
        .subcommand(ClapCommand::new("script").arg(Arg::new("filename")))
        .subcommand(
            ClapCommand::new("login")
                .arg(
                    Arg::new("oauth").short('o').long("oauth")
                        .default_value("http://localhost:8001/auth/realms/master/protocol/openid-connect"),
                )
                .arg(Arg::new("user").required(true).default_value("hyperuser"))
                .arg(
                    Arg::new("password")
                        .required(true)
                        .default_value("password"),
                ),
        )
        .allow_external_subcommands(true)
        .get_matches();

    let host = matches.get_one::<String>("host").unwrap().clone();
    let certs = matches.get_one::<String>("certs").unwrap().clone();

    if matches.subcommand_name().is_some() {
        match matches.subcommand().unwrap() {
            ("login", args) => {
                let hostname = matches.value_of("host").unwrap();
                let oauth_url = args.value_of("oauth").unwrap();
                let username = args.value_of("user").unwrap();
                let password = args.value_of("password").unwrap();
                login(hostname, oauth_url, username, password)
                    .await
                    .unwrap();
                Ok(())
            }
            ("script", args) => {
//                refresh().await?;
                let filename: &String = args.get_one("filename").unwrap();
                let script = fs::read_to_string(filename)?;
                let lines: Vec<String> = result(separated_list0(tag(";"), is_not(";"))(new_span(
                    script.as_str(),
                )))?
                .into_iter()
                .map(|i| i.to_string())
                .filter(|i| !i.trim().is_empty())
                .collect();
                let session = Session::new(host, certs).await?;
                for line in lines {
                    session.command(line.as_str()).await?;
                }

                Ok(())
            }
            (subcommand, args) => {
//                refresh().await?;
                let session = Session::new(host, certs).await?;
                session.command(subcommand).await
            }
        }
    } else {
        Ok(())
========

    let certs = match args.certs.as_ref() {
        None => format!("{}/localhost/certs", STARLANE_HOME.to_string()),
        Some(certs) => certs.clone(),
    };

    let host = match args.host.as_ref() {
        None => "localhost".to_string(),
        Some(host) => host.clone(),
    };


    let session = Session::new(host, certs).await?;

    let mut rl = rustyline::DefaultEditor::new().unwrap();
    rl.add_history_entry(history_log.as_str());
    rl.save_history(history_log.as_str());

    loop {

        let line = rl.readline(">> ").unwrap();
        rl.add_history_entry(history_log.as_str());

        let line_str = line.trim();

        if "exit" == line_str {
            return Ok(());
        }

        if line_str.len() > 0 {
            session.command(line.as_str()).await?;
        }
>>>>>>>> release/0.3.20:rust/starlane/src/cli.rs
    }

}

async fn login(host: &str, oauth_url: &str, username: &str, password: &str) -> Result<(), CliErr> {
    let mut form = HashMap::new();
    form.insert("username", username);
    form.insert("password", password);
    form.insert("client_id", "admin-cli");
    form.insert("grant_type", "password");
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/token",oauth_url))
        .form(&form)
        .send()
        .await?
        .json::<LoginResp>()
        .await?;

    {
        let mut config = crate::cli::CLI_CONFIG.lock()?;
        config.hostname = host.to_string();
        config.refresh_token = Some(res.refresh_token);
        config.oauth_url = Some(oauth_url.to_string());
        config.save()?;
    }

    Ok(())
}

async fn refresh() -> Result<String,SpaceErr> {
    let mut config= {
        let config = crate::cli::CLI_CONFIG.lock()?;
        (*config).clone()
    };

    let oauth = config.oauth_url.ok_or::<SpaceErr>("oauth login not set".into())?;
    let refresh_token = config.refresh_token.ok_or::<SpaceErr>("refresh token not set".into())?;

    let client = reqwest::Client::new();
    let url = format!("{}/token", oauth);
    let response = client
        .post(&url)
        .form(&json!({
                "refresh_token": refresh_token,
                "client_id": "admin-cli",
                "grant_type": "refresh_token"
            }))
        .send()
        .await.unwrap();

    match &response.status().as_u16() {
        200 => {
            let response: AccessTokenResp = response.json().await.unwrap();
            Ok(response.access_token)
        }
        other => {
            let response = response.text().await.unwrap();
            println!("response: {}", response);
            Err("could not refresh token".into())
        }
    }

}

pub struct Session {
    pub client: ControlClient,
    pub cli: ControlCliSession,
}

impl Session {
    pub async fn new(host: String, certs: String) -> Result<Self, SpaceErr> {
        let logger = logger!(Point::from_str("starlane-cli")?);
        let tcp_client: Box<dyn HyperwayEndpointFactory> = Box::new(HyperlaneTcpClient::new(
            host,
            certs,
            Knock::default(),
            false,
            logger,
        ));

        let client = ControlClient::new(tcp_client)?;

        client.wait_for_ready(Duration::from_secs(30)).await?;
        client.wait_for_greet().await?;

        let cli = client.new_cli_session().await?;

        Ok(Self { client, cli })
    }

    async fn command(&self, command: &str) -> Result<(), SpaceErr> {
        let blocks = result(upload_blocks(new_span(command)))?;
        let mut command = RawCommand::new(command.to_string());
        for block in blocks {
            let path = block.name.clone();
            let metadata = std::fs::metadata(&path)?;

            let content = if metadata.is_dir() {
                let file = Cursor::new(Vec::new());

                let walkdir = WalkDir::new(&path);
                let it = walkdir.into_iter();

                let data = match zip_dir(
                    &mut it.filter_map(|e| e.ok()),
                    &path,
                    file,
                    zip::CompressionMethod::Deflated,
                ) {
                    Ok(data) => data,
                    Err(e) => return Err(SpaceErr::new(500, e.to_string())),
                };

                // return the inner buffer from the cursor
                let data = data.into_inner();
                data
            } else {
                std::fs::read(block.name.as_str())?
            };

            command
                .transfers
                .push(CmdTransfer::new(block.name, content));
        }

        let core = self.cli.raw(command).await?;
        self.core_out(core);

        Ok(())
    }

    pub fn core_out(&self, core: ReflectedCore) {
        match core.is_ok() {
            true => self.out(core.body),
            false => {
                if core.body != Substance::Empty {
                    self.out(core.body);
                } else {
                    self.out_err(core.ok_or().unwrap_err());
                    std::process::exit(1);
                }
            }
        }
    }

    pub fn out(&self, substance: Substance) {
        match substance {
            Substance::Empty => {
                println!("Ok");
            }
            Substance::Err(err) => {
                println!("{}", err.to_string());
            }
            Substance::List(list) => {
                for i in list.list {
                    self.out(*i);
                }
            }
            Substance::Point(point) => {
                println!("{}", point.to_string());
            }
            Substance::Surface(surface) => {
                println!("{}", surface.to_string());
            }
            Substance::Text(text) => {
                println!("{}", text);
            }
            Substance::Stub(stub) => {
                println!("{}<{}>", stub.point.to_string(), stub.kind.to_string())
            }
            Substance::Details(details) => {
                println!(
                    "{}<{}>",
                    details.stub.point.to_string(),
                    details.stub.kind.to_string()
                )
            }
            what => {
                eprintln!(
                    "cosmic-cli not sure how to output {}",
                    what.kind().to_string()
                )
            }
        }
    }

    pub fn out_err(&self, err: SpaceErr) {
        eprintln!("{}", err.to_string())
    }
}

fn zip_dir<T>(
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
