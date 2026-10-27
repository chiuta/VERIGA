# Imagine minimă cu CLI-ul static (musl):  docker build -t veriga .
FROM rust:1.97-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p veriga-cli

FROM scratch
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /src/target/release/veriga /veriga
ENTRYPOINT ["/veriga"]
