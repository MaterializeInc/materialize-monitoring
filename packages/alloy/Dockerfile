# Minimal distroless image for running alloy in materialize-monitoring.
# This is more or less a direct fork of https://github.com/grafana/alloy/blob/main/Dockerfile
# To build this, run:
#    make alloy-image
#
# divergences are listed below:
# * We download via target (since we aren't in the alloy repo)
# * We don't rely on the alloy build environment (hard to track)
# * Only release builds are supported
# * We default to non-root
# * We do not provide an `otelcol` program
# * We add a default volume (expected to be overwritten)

## NodeJS build stage for alloy's web UI
FROM --platform=$BUILDPLATFORM node:26-trixie-slim AS ui-build

# renovate: datasource=github-releases depName=grafana/alloy
ARG ALLOY_VERSION=v1.17.0

ADD --keep-git-dir=true https://github.com/grafana/alloy.git#${ALLOY_VERSION} /src/alloy

WORKDIR /src/alloy/internal/web/ui

RUN --mount=type=cache,target=/root/.npm,sharing=locked \
    npm ci --no-audit --no-fund \
    && npm run build

## Go build stage for static binary alloy
FROM --platform=$BUILDPLATFORM golang:1.26 AS go-build

ARG BUILDARCH
# renovate: datasource=github-releases depName=grafana/alloy
ARG ALLOY_VERSION=v1.17.0

RUN export DEBIAN_FRONTEND="noninteractive" \
    && dpkg --add-architecture amd64 \
    && dpkg --add-architecture arm64 \
    && apt-get update \
    && apt-get install -qy --no-install-recommends \
        patch \
        crossbuild-essential-amd64 \
        crossbuild-essential-arm64 \
        libsystemd-dev:amd64 \
        libsystemd-dev:arm64 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

ADD --keep-git-dir=true https://github.com/grafana/alloy.git#${ALLOY_VERSION} /src/alloy

WORKDIR /src/alloy

# HACK: apply patches that are not present upstream
COPY patches/ /tmp/patches/
RUN patch -p1 < /tmp/patches/0001-do-not-clear-goos-goarch.patch

COPY --from=ui-build /src/alloy/internal/web/ui/dist /src/alloy/internal/web/ui/dist

ARG TARGETOS
ARG TARGETPLATFORM
ARG TARGETARCH
ARG TARGETVARIANT

ENV GCC_TARGET1=${TARGETARCH/arm64/aarch64}
ENV GCC_TARGET=${GCC_TARGET1/amd64/x86_64}-linux-gnu

RUN --mount=type=cache,target=/root/.cache/go-build \
    --mount=type=cache,target=/go/pkg/mod \
    RELEASE_BUILD=1 VERSION="${ALLOY_VERSION}" \
    CC="${GCC_TARGET}-gcc" \
    PKG_CONFIG_PATH="/usr/lib/${GCC_TARGET}/pkgconfig" \
    PKG_CONFIG_LIBDIR="/usr/lib/${GCC_TARGET}/pkgconfig" \
    GOOS="$TARGETOS" GOARCH="$TARGETARCH" GOARM=${TARGETVARIANT#v} \
    GO_TAGS="netgo embedalloyui promtail_journal_enabled" \
    SKIP_UI_BUILD=1 \
    make alloy

## Supporting libraries
FROM debian:trixie AS deb-support

# Standard alloy uid/username
ARG UID="473"
ARG USERNAME="alloy"

RUN export DEBIAN_FRONTEND="noninteractive" \
    && apt-get update \
    && apt-get upgrade -y \
    && apt-get install -qy --no-install-recommends \
        libsystemd0 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Seed with distroless user/group files
COPY --from=gcr.io/distroless/base-debian13 /etc/passwd /etc/group /etc/

# Create alloy user in container, but do not set it as default
RUN groupadd --gid "$UID" "$USERNAME" \
    && useradd -r -s /usr/sbin/nologin -u "$UID" -g "$UID" "$USERNAME" \
    && mkdir -p /var/lib/alloy/data \
    && chown -R "$USERNAME:$USERNAME" /var/lib/alloy \
    && chmod -R 770 /var/lib/alloy

## Final build image
FROM gcr.io/distroless/base-debian13

ARG TARGETARCH
ARG LINUX_TAG1=${TARGETARCH/arm64/aarch64}
ARG LINUX_TAG=${LINUX_TAG1/amd64/x86_64}-linux-gnu

# renovate: datasource=github-releases depName=grafana/alloy
ARG ALLOY_VERSION=v1.17.0
# Standard alloy uid/username
ARG UID="473"
ARG USERNAME="alloy"
ENV ALLOY_DEPLOY_MODE=docker

USER ${UID}:${UID}

LABEL org.opencontainers.image.source="https://github.com/grafana/alloy" \
    org.opencontainers.image.version="${ALLOY_VERSION}" \
    org.opencontainers.image.licenses="Apache-2.0" \
    org.opencontainers.image.vendor="Materialize, Inc." \
    org.opencontainers.image.title="Materialize Monitoring Alloy" \
    org.opencontainers.image.documentation="https://materializeinc.github.io/materialize-monitoring/reference/images/alloy/" \
    org.opencontainers.image.description="Distroless Grafana Alloy for Materialize Monitoring" \
    org.opencontainers.image.base.name="gcr.io/distroless/base-debian13"

COPY --from=go-build --chown=${UID}:${UID} /src/alloy/build/alloy /usr/local/bin/alloy
# This is just a stub from upstream. it is expected to use a config from our pipelines.
COPY --chown=${UID}:${UID} example-config.alloy /etc/alloy/config.alloy

COPY --from=deb-support /usr/lib/${LINUX_TAG}/libsystemd.so.0* /usr/lib/${LINUX_TAG}/
COPY --from=deb-support /etc/passwd /etc/group /etc/
COPY --chown="${UID}:${UID}" --from=deb-support /var/lib/alloy/data /var/lib/alloy/data

VOLUME ["/var/lib/alloy"]

ENTRYPOINT ["/usr/local/bin/alloy"]
CMD ["run", "/etc/alloy/config.alloy", "--storage.path=/var/lib/alloy/data"]
