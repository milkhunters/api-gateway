use std::collections::HashMap;

use actix_web::HttpResponse;
use serde::Serialize;
use tonic::transport::Channel;

use crate::proto;
use crate::proto::ums_control_client::UmsControlClient;

#[derive(Debug, Serialize)]
struct Payload {
    session_id: String,
    user_id: String,
    user_state: String,
    permissions: HashMap<String, Vec<String>>
}


pub async fn process(
    client: UmsControlClient<Channel>, 
    session_token: &str,
    user_agent: &str,
    ip: &str
) -> Result<(String, String), HttpResponse> {
    let mut client = client;
    let request = tonic::Request::new(proto::ExtractPayloadRequest {
        session_token: session_token.to_string(),
        user_agent: user_agent.to_string(),
        user_ip: ip.to_string(),
    });

    let response = client.extract_payload(request).await;
    
    match response {
        Ok(response) => {
            let model = Payload {
                session_id: response.get_ref().session_id.clone(),
                user_id: response.get_ref().user_id.clone(),
                user_state: response.get_ref().user_state.clone(),
                permissions: response.get_ref().permissions.iter().map(|(k, v)| {
                    (k.clone(), v.permission_text_ids.clone())
                }).collect()
            };
            Ok(("payload".to_string(), serde_json::to_string(&model).unwrap()))
        },
        Err(error) => {
            return match error.code() {
                tonic::Code::Unauthenticated => {
                    let mut http_error = HttpResponse::Unauthorized()
                        .body(error.message().to_string());
                    http_error.headers_mut().insert(
                        "Content-Type".parse().unwrap(),
                        "application/json".parse().unwrap()
                    );
                    Err(http_error)
                },
                _ => {
                    Err(
                        HttpResponse::InternalServerError()
                            .body(serde_json::to_string(&error.message()).unwrap())
                    )
                }
            }
        }
    }
}