use actix_web::{HttpRequest, HttpResponse, web};
use awc::error::SendRequestError;
use log::error;
use url::Url;
use crate::config::Service;

pub async fn process(
    client: &awc::Client,
    service: (&String, &Service),
    req: HttpRequest,
    payload: web::Payload,
    auth_headers: Option<(String, String)>,
    is_intermediate: bool
) -> HttpResponse {

    let service_urls = &service.1.upstreams;
    let service_name = service.0;

    let mut url = match Url::parse(&service_urls[0]) {
        Ok(value) => value,
        Err(error) => {
            error!(
                "Failed to parse URL for service {}: {:?}",
                service_name,
                error
            );
            std::process::exit(1);
        },
    };

    url = url.join(req.path().trim_start_matches('/')).unwrap();
    url.set_query(Some(req.query_string()));
    
    let conn_info = req.connection_info().clone();
    let remote_addr = {
        if is_intermediate {
            conn_info.realip_remote_addr().clone()
        } else {
            conn_info.peer_addr().clone()
        }
    };
    
    if remote_addr.is_none() {
        return HttpResponse::BadRequest().body("Failed to get remote address");
    }
    
    // TODO: balanced upstreams
    let mut request = client
        .request_from(url.as_str(), req.head())
        .insert_header(("Forwarded", format!("for={}", remote_addr.unwrap())))
        .no_decompress();

    if let Some((key, value)) = auth_headers {
        request = request.insert_header((key, value));
    }
    
    // request.headers().iter().for_each(|(h, v)| {
    //     info!("{}: {}", h, v.to_str().unwrap());
    // });

    let res = match request.send_stream(payload).await {
        Ok(res) => res,
        Err(error) => return match error {
            SendRequestError::Timeout => {
                HttpResponse::GatewayTimeout().finish()
            },  
            _ => {
                HttpResponse::BadGateway().body(error.to_string())
            },
        },
    };

    let mut client_response = HttpResponse::build(res.status());

    // TODO: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#directives
    for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection") {
        client_response.insert_header((header_name.clone(), header_value.clone()));
    }
    client_response.streaming(res)
}