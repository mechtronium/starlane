use std::collections::HashMap;
use std::str::FromStr;

use std::thread;


use url::Url;

use crate::star::{StarSkel};
use crate::starlane::api::{StarlaneApi, StarlaneApiRelay};
use tokio::sync::{oneshot, mpsc};
use crate::star::variant::{VariantCall, FrameVerdict};
use crate::util::{AsyncRunner, AsyncProcessor};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::error::Error;
use bytes::BytesMut;
use httparse::{Request, Header};
use std::sync::Arc;
use std::convert::TryInto;
use handlebars::Handlebars;
use serde_json::json;
use std::future::Future;
use mesh_portal_serde::version::latest::entity::request::Http;
use mesh_portal_serde::version::latest::http::HttpResponse;
use mesh_portal_serde::version::latest::id::Meta;
use mesh_portal_serde::version::latest::payload::{HttpMethod, Payload, Primitive};
use nom::AsBytes;
use crate::artifact::ArtifactRef;
use crate::cache::ArtifactItem;
use crate::html::HTML;
use regex::Regex;
use crate::resource::ArtifactKind;
use crate::resources::message::ProtoRequest;
use serde::{Serialize,Deserialize};
use crate::star::variant::web::parse::host_and_port;


pub struct WebVariant {
    skel: StarSkel,
}

impl WebVariant {
    pub fn start(skel: StarSkel, rx: mpsc::Receiver<VariantCall>) {
        AsyncRunner::new(
            Box::new(Self { skel: skel.clone() }),
            skel.variant_api.tx.clone(),
            rx,
        );
    }
}

#[async_trait]
impl AsyncProcessor<VariantCall> for WebVariant {
    async fn process(&mut self, call: VariantCall) {
        match call {
            VariantCall::Init(tx) => {
                self.init_web(tx);
            }
            VariantCall::Frame { frame, session:_, tx } => {
                tx.send(FrameVerdict::Handle(frame));
            }
        }
    }
}


impl WebVariant {
    fn init_web(&self, tx: tokio::sync::oneshot::Sender<Result<(), crate::error::Error>>) {
        let api = StarlaneApi::new(self.skel.surface_api.clone(), self.skel.info.address.clone() );

        start(api,self.skel.clone());

        tx.send(Ok(())).unwrap_or_default();
    }
}

fn start(api: StarlaneApi,skel: StarSkel) {
    thread::spawn(move || {

        let runtime = Runtime::new().unwrap();
        runtime.block_on( async move {

            match std::net::TcpListener::bind("127.0.0.1:8080") {
                Ok(std_listener) => {
                    let listener = TcpListener::from_std(std_listener).unwrap();
                    while let Ok((mut stream, _)) = listener.accept().await {
                        let api = api.clone();
                        let skel = skel.clone();
                        tokio::task::spawn_blocking(move || {
                            tokio::spawn(async move {
                                match process_request(stream, api.clone(), skel).await {
                                    Ok(_) => {
                                        info!("ok");
                                    }
                                    Err(error) => {
                                        error!("{}",error);
                                    }
                                }
                            });
                        });
                    }
                }
                Err(error) => {
                    error!("FATAL: could not setup TcpListener {}", error);
                }
            }
        });
    });
}

async fn process_request( mut stream: TcpStream, api: StarlaneApi, skel: StarSkel ) -> Result<(),Error>{
    info!("received HTTP Stream...");

    let mut request_buf: Vec<u8> = vec![];
    let mut buf = [0 as u8; 16384]; // 16k read buffer

    let request = loop {
        match stream.read(&mut buf).await {
            Ok(size) => request_buf.extend(&buf[0..size]),
            Err(_) => {} // handle err,
        }
println!("ok...");
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = Request::new(&mut headers);
        if let Ok(status) = req.parse(&request_buf) {

            if status.is_complete() {

                let mut http_headers = Meta::new();
                for header in req.headers {
                    http_headers.insert(header.name.to_string(), String::from_utf8(header.value.to_vec())?);
                }

info!("method: {}", req.method.expect("method"));
                let method = HttpMethod::from_str(req.method.expect("expected method"))?;

                let body_offset = status.unwrap();
                let mut body:Vec<u8> = vec![];
                for index in body_offset..request_buf.len() {
                    body.push( request_buf.get(index).unwrap().clone() );
                }
                let body =  Option::Some(Arc::new(body));

                break Http{
                    path: req.path.expect("expected path").to_string(),
                    method: method,
                    headers: http_headers,
                    body
                };
            } else {
                println!("incomplete parse... ");
            }
        }
    };

    match create_response(request,api,skel).await {
        Ok(response) => {
            stream.write(format!("HTTP/1.1 {} OK\r\n\r\n",response.code).as_bytes() ).await?;

            if response.body.is_some() {
                stream.write( response.body.expect("expected response body").as_bytes() ).await?;
            }
        }
        Err(e) => {
eprintln!("ERROR: {}", e.to_string() );
            error_response(stream, 500, "Internal Server Error").await;
        }
    }

    Ok(())
}

async fn error_response( mut stream: TcpStream, code: usize, message: &str)  {
    stream.write(format!("HTTP/1.1 {} OK\r\n\r\n",code).as_bytes() ).await.unwrap();
    let messages = json!({"title": code, "message":message});
    stream.write(HTML.render("error-code-page", &messages ).unwrap().as_bytes() ).await.unwrap();
}

async fn create_response( request: Http, api: StarlaneApi, skel: StarSkel ) -> Result<HttpResponse,Error> {

    // 



    let host_and_port = host_and_port(request.headers.get("Host").ok_or("Missing HOST")?.as_str())?.1;
    let error = format!("Space '{}' has not been created.", host_and_port.host );
    let messages = json!({"title": "ERROR", "message": error});
    let body  = HTML.render("error-code-page", &messages )?;
    let body = Option::Some(Arc::new(body.as_bytes().to_vec()));

    let response = HttpResponse {
        code: 404,
        body,
        headers: Default::default()
    };

    Ok(response)
}



mod tests {

}
#[cfg(test)]
mod test {
    use crate::error::Error;
    use regex::Regex;
    use crate::star::variant::web::parse::host_and_port;

    #[test]
    pub fn path_regex() -> Result<(),Error> {
        let regex = Regex::new("/files/")?;
        assert!(regex.is_match("/files/"));


        let regex = Regex::new("/files/.*")?;
        assert!(regex.is_match("/files/"));

        let regex = Regex::new("/files/(.*)")?;
        assert!(regex.is_match("/files/some-path"));
        assert_eq!("/some-path".to_string(),regex.replace("/files/some-path", "/$1").to_string());


        let regex = Regex::new("/files/(.*)")?;
        assert!(regex.is_match("/files/some/path.html"));
        assert_eq!("/some/path.html".to_string(),regex.replace("/files/some/path.html", "/$1").to_string());
        Ok(())
    }

    #[test]
    pub fn host() -> Result<(),Error> {
        let (_,host_and_port) = host_and_port("localhost:8080")?;
        assert_eq!( host_and_port.host, "localhost".to_string() );
        assert_eq!( host_and_port.port, 8080 );
        Ok(())
    }
}

pub struct HostAndPort {
    pub host: String,
    pub port: u32
}

pub mod parse {
    use std::num::ParseIntError;
    use std::str::FromStr;
    use mesh_portal_versions::version::v0_0_1::parse::{domain, Res};
    use nom::bytes::complete::{is_a, tag, take_while};
    use nom::character::is_digit;
    use nom::error::{ErrorKind, ParseError, VerboseError};
    use nom::sequence::tuple;
    use crate::star::variant::web::HostAndPort;

    pub fn host_and_port(input: &str ) -> Res<&str, HostAndPort> {
        let (next, (host,_,port)) = tuple(( domain, tag(":"), is_a("0123456789")  ) )(input)?;

        let host = host.to_string();
        let port: &str = port;
        let port = match u32::from_str(port) {
            Ok(port) => port,
            Err(err) => {
                return Err(nom::Err::Error(VerboseError::from_error_kind(
                    input,
                    ErrorKind::Tag,
                )))
            }
        };
        let host_and_port = HostAndPort {
            host,
            port
        };
        Ok((next, host_and_port))
    }

}
