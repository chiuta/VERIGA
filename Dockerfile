# Imagine minimă cu CLI-ul static (musl):  docker build -t veriga .
#   docker run --rm ghcr.io/chiuta/veriga check https://example.org
#   docker run --rm -v "$PWD/data:/data" ghcr.io/chiuta/veriga watch --once --config /data/guardian.json
FROM rust:1.97-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p veriga-cli \
 && mkdir /data && chown 65532:65532 /data

FROM scratch
LABEL org.opencontainers.image.title="VERIGA" \
      org.opencontainers.image.description="Verificator de legături (Xenu-like) — CLI static" \
      org.opencontainers.image.source="https://github.com/chiuta/VERIGA" \
      org.opencontainers.image.url="https://veriga.chiuta.ro" \
      org.opencontainers.image.licenses="MIT"
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /src/target/release/veriga /veriga
COPY --from=build --chown=65532:65532 /data /data
USER 65532:65532
WORKDIR /data
VOLUME ["/data"]
ENTRYPOINT ["/veriga"]
