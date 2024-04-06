use actix_web::{HttpRequest, HttpResponse, Result, web};

use crate::AppState;

pub fn router(cfg: &mut web::ServiceConfig) {
    cfg.default_service(
        web::route().to(http_gateway)
    );
}

async fn http_gateway(
    tgr: HttpRequest,
    app_state: web::Data<AppState>
) -> Result<HttpResponse> {
    let method = tgr.method().clone();
    let connection_info = tgr.connection_info();

    let input_uri_str = &format!(
        "{}{}", connection_info.host(), match tgr.uri().path_and_query() {
            Some(path) => path.as_str(),
            None => "",
        }
    );

    let service = match app_state.service_matching.iter()
        .find(|(_, re)| re.is_match(input_uri_str))
        .map(|(service_name, _)| service_name)
    {
        Some(name) => match app_state.config.services.get(name) {
            Some(service) => (name, service),
            None => return Ok(
                HttpResponse::InternalServerError().body("Service matched, but not found 0_o")
            ),
        }
        None => return Ok(HttpResponse::NotFound().body("Service not found")),
    };

    let client = &app_state.http_client;

    let mut fgr = client.request(method, &service.1.upstreams[0]);
    for (key, value) in tgr.headers().iter() {
        fgr = fgr.insert_header((key.clone(), value.clone()));
    }
    fgr = fgr.insert_header(
        ("X-Forwarded-For", connection_info.host().to_string()),
    );

    let mut res = match fgr.send().await {
        Ok(res) => res,
        Err(error) => {
            return Ok(HttpResponse::InternalServerError().body(error.to_string()));
        },
    };

    let body = match res.body().await {
        Ok(body) => body,
        Err(error) => {
            return Ok(HttpResponse::InternalServerError().body(error.to_string()));
        },
    };


    Ok(HttpResponse::Ok().body(body))
}
