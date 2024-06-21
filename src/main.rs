use std::net::TcpListener;
use std::thread;
use actix_web::{
    App,
    HttpServer,
    middleware::Logger,
    web
};
use dotenv::dotenv;

use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use regex::Regex;

use crate::proto::ums_control_client::UmsControlClient;

pub mod proto {
    tonic::include_proto!("ums.control");
}


mod config;
mod core;

#[derive(Clone)]
struct AppState {
    config: config::Config,
    service_matching: Vec<(String, Regex)>,
    client: awc::Client,
    grpc_client: Option<UmsControlClient<tonic::transport::Channel>>
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let config = match config::Config::new("config.yaml") {
        Ok(config) => config,
        Err(error) => {
            log::error!("Failed to load config: {}", error);
            std::process::exit(1);
        },
    };

    if let Some(log_level) = &config.log_level {
        std::env::set_var("RUST_LOG", log_level);
    }

    env_logger::builder()
        .filter_module("consulrs", log::LevelFilter::Error)
        .filter_module("tracing", log::LevelFilter::Error)
        .filter_module("rustify", log::LevelFilter::Error)
        .init();


    let host = config.host.clone();
    let port = config.port;
    let tls = config.tls.clone();

    let workers = config.workers.unwrap_or(
        match thread::available_parallelism() {
            Ok(parallelism) => usize::from(parallelism),
            Err(_) => 1,
        }
    );

    let grpc_ums_client = {
        if let Some(auth) = &config.auth {
            if auth.grpc_host.is_empty() || auth.grpc_port == 0 {
                log::error!("Auth config is invalid");
                std::process::exit(1);
            }

            match UmsControlClient::connect(
                format!(
                    "{protocol}://{host}:{port}",
                    protocol = "http",
                    host = auth.grpc_host,
                    port = auth.grpc_port
                )
            ).await {
                Ok(client) => Some(client),
                Err(error) => {
                    log::error!("Failed to connect to gRPC server: {}", error);
                    std::process::exit(1);
                },
            }
        } else {
            None
        }
    };


    let mut service_matching = Vec::new();
    for (service_name, service) in config.services.iter() {
        let re = match Regex::new(&service.url_match) {
            Ok(re) => re,
            Err(error) => {
                log::error!("Failed to compile regex for service {}: {}", service_name, error);
                std::process::exit(1);
            },
        };
        service_matching.push((service_name.clone(), re));
    }

    let app_builder = move || {
        App::new()
            .app_data(
                web::Data::new(AppState {
                    config: config.clone(),
                    service_matching: service_matching.clone(),
                    grpc_client: grpc_ums_client.clone(),
                    client: awc::Client::new(),
                })
            )
            .configure(core::usecase::router)
            // .wrap(Logger::new("[%s] [%{r}a] %U"))
    };

    let listener = match TcpListener::bind(format!("{}:{}", host, port)) {
        Ok(listener) => {
            listener
        },
        Err(error) => {
            log::error!("Failed to bind to port {} in host {}: {}", host, port, error);
            std::process::exit(1);
        },
    };

    let mut server = HttpServer::new(app_builder);
    if let Some(tls) = tls {
        let mut tls_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        match tls_builder.set_private_key_file(tls.key, SslFiletype::PEM) {
            Ok(_) => {},
            Err(error) => {
                log::error!("Failed to set private key file: {}", error.to_string());
                std::process::exit(1);
            },
        }
        match tls_builder.set_certificate_chain_file(tls.cert) {
            Ok(_) => {},
            Err(error) => {
                log::error!("Failed to set certificate chain file: {}", error.to_string());
                std::process::exit(1);
            },
        };
        server = server.listen_openssl(listener, tls_builder).unwrap();
    } else {
        server = server.listen(listener).unwrap();
    }

    server.addrs_with_scheme().iter().for_each(|addr| {
        let (socket_addr, str_ref) = addr;
        log::info!("🚀 Http Server started at {}://{:?}", str_ref, socket_addr);
    });
    server.workers(workers).run().await.map(|_| {
        log::info!("Http Server stopped!")
    })?;

    Ok(())
}
