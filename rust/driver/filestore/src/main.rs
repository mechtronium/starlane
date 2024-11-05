use clap::{Parser, Subcommand};
use std::io;
use std::{env, fs, path::PathBuf, process};
use strum_macros::EnumString;

use std::fs::File;
use std::io::prelude::*;
use std::io::stdin;
use std::path::Path;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, EnumString, strum_macros::Display)]
enum Commands {
    Init,
    Write {path: PathBuf },
    Read {path: PathBuf},
    Mkdir{ path: PathBuf },
    Delete { path: PathBuf },
    List { path: Option<PathBuf> },
    Exists{ path: PathBuf },
    Pwd,
    Test
}

fn main() -> Result<(),()> {
    let cli = Cli::parse();

    let pwd = env::var("PWD").unwrap_or(".".to_string());


    match cli.command {
        Commands::Write { path } => {
            let mut file = File::create(path ).unwrap();
            // Create a handle to stdin
            io::copy(&mut io::stdin(), &mut file).unwrap();
        }
        Commands::Read { path } => {

            let mut file = File::open(path).unwrap();
            let mut buf : [u8;1024] = [0; 1024];
            while let Ok(size) = file.read(& mut buf) {
                if size == 0 {
                    break;
                }
                io::stdout().write( & buf ).unwrap();
            }
        }
        Commands::Mkdir { path } => {
            fs::create_dir(path).unwrap();
        }
        Commands::Delete { path } => {
            fs::remove_file(path).unwrap();
        }
        Commands::List { path } => {

            let path = match &path {
                None => PathBuf::from(pwd.clone()),
                Some(path) => path.clone()
            };

            let paths = fs::read_dir(path).unwrap();


            for path in paths {
                let path = path.unwrap().path();
            }
        }
    Commands::Pwd =>  {
        //println!("{}", pwd);
    }
        Commands::Test =>  {
            let dir = Path::new("subdir");

            /*fs::metadata(dir).and_then( |m| {
              println!("! --> ./subdir already exists!");
                Ok(m)
            });
           // fs::create_dir(dir).unwrap();

             */


            let file = dir.join("file1.txt");
            let mut file = File::create(file).unwrap();
            file.write_all(b"Hello, world!").unwrap();

            let file = dir.join("file2.txt");
            let mut file = File::create(file).unwrap();
            file.write_all(b"Blah Blah blah!").unwrap();
            let paths = fs::read_dir(dir).unwrap();
            paths.into_iter().map( |d| d.unwrap() );

        }
        Commands::Exists { path } => {
            File::open(path).unwrap();
/*            match fs::exists(path) {
                Ok(_) => {
                    Ok(())
                }
                Err(_) => {
                    Err(())
                }
            }

 */
        }
    }


    Ok(())
}


#[cfg(test)]
pub mod test {

}