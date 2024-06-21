use std::collections::HashMap;
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
) -> Result<(String, String), actix_web::Error> {
    let mut client = client;
    // let start = std::time::Instant::now();
    let request = tonic::Request::new(proto::EpRequest {
        session_token: session_token.to_string(),
        user_agent: user_agent.to_string(),
        user_ip: ip.to_string(),
    });

    let response = client.extract_payload(request).await;
    
    match response {
        Ok(response) => {
            // info!("Session token is valid {}", response.get_ref().session_id.clone());
            let model = Payload {
                session_id: response.get_ref().session_id.clone(),
                user_id: response.get_ref().user_id.clone(),
                user_state: response.get_ref().user_state.clone(),
                permissions: response.get_ref().permissions.iter().map(|(k, v)| {
                    (k.clone(), v.permission.clone())
                }).collect()
            };
            // info!("Elapsed time: {:?}", start.elapsed());
            Ok(("payload".to_string(), serde_json::to_string(&model).unwrap()))
        },
        Err(error) => {
            Err(actix_web::error::ErrorUnauthorized(error))
        }
    }
}