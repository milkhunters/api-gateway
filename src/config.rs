use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tls {
    pub cert: String,
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth {
    pub grpc_host: String,
    pub grpc_port: u16
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Service {
    pub url_match: String,
    pub tls_cert: Option<String>,
    pub upstreams: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub is_intermediate: bool,
    pub log_level: Option<String>,
    pub tls: Option<Tls>,
    pub auth_servers: Option<Vec<String>>,
    pub services: HashMap<String, Service>,
}

impl Config {
    pub fn new(config_path: &str) -> Result<Config, String> {

        if !std::path::Path::new(config_path).exists() {
            let default_config = Config {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: Some(4),
                is_intermediate: false,
                log_level: Some("info".to_string()),
                tls: None,
                auth_servers: vec!["http://auth:50051".to_string()].into(),
                services: {
                    let mut services = HashMap::new();
                    services.insert(
                        "Ums".to_string(),
                        Service {
                            url_match: "(test|stage).mlkh.ru/api(1|2)/ums/.*".to_string(),
                            tls_cert: Some("service/cert.pem".to_string()),
                            upstreams: vec!["http://ums:8080".to_string()],
                        }
                    );
                    services.insert(
                        "Blog".to_string(),
                        Service {
                            url_match: "(test|stage).mlkh.ru/api(1|2)/blog/.*".to_string(),
                            tls_cert: None,
                            upstreams: vec!["http://blog:8080".to_string()],
                        }
                    );
                    services
                }
            };

            match OpenOptions::new().write(true).create_new(true).open(config_path) {
                Ok(mut file) => {
                    if let Err(error) = writeln!(file, "{}", serde_yaml::to_string(&default_config).unwrap()) {
                        return Err(format!("Failed to write to config.yaml -> {}", error));
                    }
                },
                Err(error) => return Err(format!("Failed to create config.yaml -> {}", error)),
            };
        }


        let mut file = match File::open(config_path) {
            Ok(file) => file,
            Err(error) => return Err(format!("Failed to open config.yaml -> {}", error)),
        };
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(_) => (),
            Err(error) => return Err(format!("Failed to read config.yaml -> {}", error)),
        };

        let config: Config = match serde_yaml::from_str(&contents) {
            Ok(config) => config,
            Err(error) => {
                return Err(format!("Failed to parse config.yaml -> {}", error));
            },
        };
        Ok(config)
    }
}