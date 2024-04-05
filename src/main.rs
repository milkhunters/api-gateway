use actix_web::{
    App,
    HttpServer,
    middleware::Logger,
    web
};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

mod routes;
mod models;
mod config;
mod core;

struct AppState {
    config: config::Config,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = match config::Config::new("config.yaml") {
        Ok(config) => config,
        Err(error) => {
            log::error!("Failed to load config: {}", error);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, error));
        },
    };
    let host = config.host.clone();
    let port = config.port.clone();
    let tls = config.tls.clone();

    log::info!("Starting server at http://{}:{}", host, port);

    let app_builder = move || {
        App::new().app_data(web::Data::new(AppState {
            config: config.clone(),
        })
        ).configure(routes::router).wrap(Logger::new("[%s] [%{r}a] %U"))
    };

    if tls.enabled {
        let mut tls_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        match tls_builder.set_private_key_file(tls.key, SslFiletype::PEM) {
            Ok(_) => {},
            Err(error) => {
                log::error!("Failed to set private key file: {}", error.to_string());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, error));
            },
        }
        match tls_builder.set_certificate_chain_file(tls.cert) {
            Ok(_) => {},
            Err(error) => {
                log::error!("Failed to set certificate chain file: {}", error.to_string());
                return Err(std::io::Error::new(std::io::ErrorKind::Other, error));
            },
        };
        HttpServer::new(app_builder).bind_openssl((host, port), tls_builder)?.run().await
    } else {
        HttpServer::new(app_builder).bind((host, port))?.run().await
    }
}
