# API Gateway

A simple service that processes user requests, performing authorization and balancing between services

### Todo

- [X] HTTP Gateway
- [ ] WS Gateway
- [X] Tls Gateway (testing)
- [ ] Tls Services
- [X] Auth Middleware integration
- [ ] Cutter Middleware integration
- [X] Logger Middleware integration
- [X] Balancer

### Config example

The config file is created automatically if it was not created manually

```yaml
workers: 4
is_intermediate: false
log_level: info
tls:
  cert: cert/cert.pem
  key: cert/key.pem
auth_servers:
  - http://127.0.0.2:50051
  - http://127.0.0.3:50051
services:
  SomeService:
    url_match: (test|stage).mlkh.ru:8080/.*
    upstreams:
    - http://127.0.0.2:8081/
    - http://127.0.0.3:8081/
```