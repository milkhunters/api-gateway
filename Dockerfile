# Build image
FROM rust:1.79.0-alpine3.20 as build


RUN apk add --no-cache build-base musl-dev protoc protobuf-dev libressl-dev

WORKDIR /usr/service
COPY Cargo.toml Cargo.lock ./

# Build and cache the dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN cargo build --release
RUN rm src/main.rs

# Copy the actual code files and build the application
COPY src ./src
COPY proto ./proto
COPY build.rs .

# Update the file date
RUN touch src/main.rs
RUN cargo build --release


# Runtime image
FROM alpine:3.20


RUN apk add --no-cache openssl

WORKDIR /usr/local/bin

COPY --from=build /usr/service/target/release/api-gateway .
COPY *.yaml .

CMD ["api-gateway"]
