# Runtime container image for bbb-server, bbb-admin, and bbb-web-host (with prebuilt WASM bundle).
# Built in CI from a staged context containing:
#   bin/bbb-server, bin/bbb-admin, bin/bbb-web-host, web-dist/, LICENSE-MIT, LICENSE-APACHE
ARG BASE_IMAGE=ubuntu:26.04
FROM ${BASE_IMAGE}

LABEL org.opencontainers.image.source="https://github.com/recursion128/bbb" \
      org.opencontainers.image.description="BBB Minecraft Java 26.1 dedicated server, admin CLI, and WebGPU client gateway" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"

RUN DEBIAN_FRONTEND=noninteractive apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /var/lib/bbb --shell /usr/sbin/nologin bbb \
    && mkdir -p /var/lib/bbb/world /var/lib/bbb/admin /var/lib/bbb/vanilla /var/cache/bbb-web /opt/bbb/web-dist /usr/share/licenses/bbb \
    && chown -R bbb:bbb /var/lib/bbb /var/cache/bbb-web

COPY bin/bbb-server bin/bbb-admin bin/bbb-web-host /usr/local/bin/
COPY web-dist/ /opt/bbb/web-dist/
COPY LICENSE-MIT LICENSE-APACHE /usr/share/licenses/bbb/

RUN chmod 0755 /usr/local/bin/bbb-server /usr/local/bin/bbb-admin /usr/local/bin/bbb-web-host

ENV BBB_WEB_OUT_DIR=/opt/bbb/web-dist \
    BBB_VANILLA_ROOT=/var/lib/bbb/vanilla

WORKDIR /var/lib/bbb
USER bbb

EXPOSE 25565/tcp 8080/tcp

CMD ["/usr/local/bin/bbb-server", "--bind", "0.0.0.0:25565", "--world", "/var/lib/bbb/world", "--admin-dir", "/var/lib/bbb/admin"]
