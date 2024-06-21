use actix_web::{HttpRequest, HttpResponse, Result, web};

use crate::AppState;
use crate::core::{auth, proxy};

pub fn router(cfg: &mut web::ServiceConfig) {
    cfg.default_service(
        web::route().to(http_gateway)
    );
}


async fn http_gateway(
    req: HttpRequest,
    payload: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {

    // Service Matching Step
    let req_uri_str = &format!(
        "{}{}", req.connection_info().host().to_string(), match req.uri().path_and_query() {
            Some(path) => path.as_str(),
            None => "",
        }
    );

    let service = match app_state.service_matching.iter()
        .find(|(_, re)| re.is_match(req_uri_str))
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

    // Auth Step todo: clear user payload header if exists
    let mut auth_headers: Option<(String, String)> = None;
    if let Some(session_token) = req.cookie("session_token") {
        if let Some(grpc_client) = app_state.grpc_client.clone() {

            let user_agent = req.headers().get("user-agent").map(|value| {
                value.to_str().unwrap()
            });
            
            let ip = {
                if app_state.config.is_intermediate {
                    req.connection_info().realip_remote_addr().unwrap().to_string()
                } else {
                    req.connection_info().peer_addr().unwrap().to_string()
                }
            };
            
            match auth::process(
                grpc_client, 
                session_token.value(),
                user_agent.unwrap_or("Unknown"),
                &ip
            ).await {
                Ok((key, value)) => {
                    auth_headers = Some((key, value));
                },
                Err(error) => return Err(error)
            }
        }
    }
    // Proxy Step
    Ok(proxy::process(
        &app_state.client, 
        service, 
        req,
        payload, 
        auth_headers,
        app_state.config.is_intermediate
    ).await)
    
}
