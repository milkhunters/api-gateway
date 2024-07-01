# Build image
FROM rust:1.79.0-alpine3.20 as build

WORKDIR /usr/service

RUN apk add --no-cache build-base musl-dev protoc protobuf-dev libressl-dev

COPY . .


RUN cargo install --path .

# Runtime image
FROM alpine:3.20.0

RUN apk add --no-cache openssl

WORKDIR /usr/local/bin

COPY --from=build /usr/local/cargo/bin/api-gateway .

CMD ["api-gateway"]