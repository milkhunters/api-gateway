use actix_web::{get, post, Responder, web};

pub fn router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/gateway")
            .default_service(
                web::route().to(gateway)
            )
    );
}

async fn gateway() -> impl Responder {
    "Hello world!"
}
