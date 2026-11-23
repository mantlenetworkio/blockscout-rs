# Base images are pinned to a full tag plus its multi-arch index digest, so the
# same commit always builds with the same toolchain. Upstream still uses
# services-base:latest; bump the pin when syncing with upstream.
FROM ghcr.io/blockscout/services-base:chef-0.1.78-rust-1.99.0-protoc-31.1-openapi-2.27.1@sha256:aa4034c0bd2cae2b34798a011fa27f271f0ca9ce753ac2bdd2b986262797e938 as chef

FROM chef AS plan
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as cache
COPY --from=plan /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM chef AS build

COPY . .
COPY --from=cache /app/target target
COPY --from=cache $CARGO_HOME $CARGO_HOME
RUN cargo build --release

# Dependabot refreshes the digest (.github/dependabot.yml).
FROM ubuntu:24.04@sha256:9c7f200c07e3d4a03e7249d2c16bf66a51120fb876aa2242cf31451412543289 as run
RUN apt-get update && apt-get install -y libssl3 libssl-dev ca-certificates

WORKDIR /app
ENV APP_USER=app
# Processes in a container should not run as root, so we need to create app user
# https://medium.com/@mccode/processes-in-containers-should-not-run-as-root-2feae3f0df3b
RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER

COPY --from=build /app/target/release/sig-provider-server /app/sig-provider-server
# Change directory access for app user
RUN chown -R $APP_USER:$APP_USER /app
USER app

CMD ["./sig-provider-server"]
