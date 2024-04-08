use actix_web::{HttpRequest, HttpResponse, Result, web};
use awc::error::SendRequestError;

use crate::AppState;

pub fn router(cfg: &mut web::ServiceConfig) {
    cfg.default_service(
        web::route().to(http_gateway)
    );
}


async fn http_gateway(
    tgr: HttpRequest,
    client: web::Data<awc::Client>,
    payload: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
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

    // TODO: balanced upstreams
    let fgr = client
        .request_from(&service.1.upstreams[0], tgr.head())
        .insert_header(("Forwarded", format!("for={}", connection_info.host())))
        .no_decompress();

    let res = match fgr.send_stream(payload).await {
        Ok(res) => res,
        Err(error) => return match error {
            SendRequestError::Timeout => {
                Ok(HttpResponse::GatewayTimeout().finish())
            },
            _ => {
                Ok(HttpResponse::BadGateway().body(error.to_string()))
            },
        },
    };

    let mut client_response = HttpResponse::build(res.status());

    // TODO: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#directives
    for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection") {
        client_response.insert_header((header_name.clone(), header_value.clone()));
    }

    Ok(client_response.streaming(res))
}
