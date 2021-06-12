
use std::net::ToSocketAddrs;

use actix_web::client::Client;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use clap::{value_t, Arg};
use url::Url;
use crate::star::StarSkel;
use crate::star::variant::StarVariant;
use crate::starlane::api::{StarlaneApi, StarlaneApiRelay};
use std::thread;
use crate::resource::ResourceAddress;
use std::str::FromStr;
use actix_web::web::Data;
use std::sync::Arc;
use crate::message::Fail;
use actix_web::http::StatusCode;


pub struct WebVariant
{
    skel: StarSkel,
}

impl WebVariant
{
    pub async fn new(skel: StarSkel) -> WebVariant
    {
        WebVariant
        {
            skel: skel.clone(),
        }
    }
}


#[async_trait]
impl StarVariant for WebVariant
{

    async fn init(&self, tx: tokio::sync::oneshot::Sender<Result<(),crate::error::Error>>) {
        let api = StarlaneApi::new(self.skel.star_tx.clone()).into();
        start(api);

        tx.send(Ok(()));
    }
}

fn start(api: StarlaneApiRelay){
    thread::spawn(|| {
                      web_server(api);
                  });
}



async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    api: web::Data<StarlaneApiRelay>,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {

    let address = ResourceAddress::from_str(format!("hyperspace:default:*:website:{}::<File>",req.path()).as_str() ).unwrap();
    let responder= match api.get_resource_state(address.into()).await{
        Ok(state) => {
            println!("State OK... getting option");
            match state {
                None => {
                    eprintln!("state was nothing!");
                    "404".to_string()
                }
                Some(state) => {
                    println!("found state!");
                    String::from_utf8((*state).clone() ).unwrap()
                }
            }
        }
        Err(err) => {
            eprintln!("Fail: {}",err.to_string());
            "500".to_string()
        }
    };
    Ok(responder.into())
}

async fn proxy(
    req: HttpRequest,
    body: web::Bytes,
    api: web::Data<StarlaneApi>,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    println!("Hello");
    let url = Data::new(Url::parse("http://starlane.io").unwrap());
    let mut new_url = url.get_ref().clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    // TODO: This forwarded implementation is incomplete as it only handles the inofficial
    // X-Forwarded-For header but not the official Forwarded one.
    let forwarded_req = client
        .request_from(new_url.as_str(), req.head())
        .no_decompress();
    let forwarded_req = if let Some(addr) = req.head().peer_addr {
        forwarded_req.header("x-forwarded-for", format!("{}", addr.ip()))
    } else {
        forwarded_req
    };

    let mut res = forwarded_req.send_body(body).await.map_err(Error::from)?;

    let mut client_resp = HttpResponse::build(res.status());
    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in
    res.headers().iter().filter(|(h, _)| *h != "connection")
    {
        client_resp.header(header_name.clone(), header_value.clone());
    }

    Ok(client_resp.body(res.body().await?))
}

#[actix_web::main]
async fn web_server(api: StarlaneApiRelay ) -> std::io::Result<()> {


    let forward_url = Url::parse("http://starlane.io")
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .data(api.clone())
            .data(forward_url.clone())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(forward))
    })
        .client_timeout(100_000)
        .bind("127.0.0.1:8080")?
        .system_exit()
        .run()
        .await
}
