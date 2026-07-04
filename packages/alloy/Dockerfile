# Minimal distroless image for running alloy in materialize-monitoring.
# This is an alternative to https://github.com/grafana/alloy/blob/main/Dockerfile
# To build this, run:
#    make alloy-image
# This uses the FIPS 140-3 compliant alloy-boringcrypto binary
# This can be used as a drop-in replacement for the upstream alloy image

## Supporting libraries
FROM debian:trixie-20260623@sha256:d07d1b51c39f51188e60be9b64e6bf769fa94e187f092bc32b91305cfa34ba5a AS deb-support

# Standard alloy uid/username
ARG UID="473"
ARG USERNAME="alloy"

# renovate: datasource=github-releases packageName=grafana/alloy
ARG ALLOY_VERSION=v1.17.0

# Seed with distroless user/group files
COPY --from=gcr.io/distroless/base-debian13@sha256:57c1e4c72feb5925c4763ae4f6bd2013ad3854f57eff5b60dd9acb1ce0abc66e /etc/passwd /etc/group /etc/

# Create alloy user in container, but do not set it as default
RUN groupadd --gid "$UID" "$USERNAME" \
    && useradd -r -s /usr/sbin/nologin -u "$UID" -g "$UID" "$USERNAME" \
    && mkdir -p /var/lib/alloy/data \
    && chown -R "$USERNAME:$USERNAME" /var/lib/alloy \
    && chmod -R 770 /var/lib/alloy

ARG TARGETARCH
ARG TARGETOS

RUN export DEBIAN_FRONTEND="noninteractive" \
    && apt-get update \
    && apt-get upgrade -y \
    && apt-get install -qy --no-install-recommends \
        libsystemd0 \
        unzip \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

ADD https://github.com/grafana/alloy/releases/download/${ALLOY_VERSION}/SHA256SUMS /tmp/SHA256SUMS
ADD https://github.com/grafana/alloy/releases/download/${ALLOY_VERSION}/alloy-boringcrypto-${TARGETOS}-${TARGETARCH}.zip /tmp/alloy.zip
RUN cd /tmp \
    && expected=$(grep "alloy-boringcrypto-${TARGETOS}-${TARGETARCH}.zip$" SHA256SUMS | cut -d' ' -f1) \
    && echo "$expected  alloy.zip" | sha256sum -c -
RUN mkdir -p /tmp/alloy \
    && unzip /tmp/alloy.zip -d /tmp/alloy/ \
    && install -m 0755 /tmp/alloy/alloy-boringcrypto-${TARGETOS}-${TARGETARCH} /usr/local/bin/alloy \
    && rm /tmp/alloy.zip

# Sanity-check the binary runs and assert it is the FIPS (boringcrypto) build,
# so a bad release asset or the non-FIPS variant fails the build loudly.
RUN alloy --version | grep boringcrypto

## Final build image
FROM gcr.io/distroless/base-debian13@sha256:57c1e4c72feb5925c4763ae4f6bd2013ad3854f57eff5b60dd9acb1ce0abc66e

ARG TARGETARCH
ARG LINUX_TAG1=${TARGETARCH/arm64/aarch64}
ARG LINUX_TAG=${LINUX_TAG1/amd64/x86_64}-linux-gnu

# renovate: datasource=github-releases packageName=grafana/alloy
ARG ALLOY_VERSION=v1.17.0
# Standard alloy uid/username
ARG UID="473"
ARG USERNAME="alloy"
ENV ALLOY_DEPLOY_MODE=docker

LABEL org.opencontainers.image.source="https://github.com/MaterializeInc/materialize-monitoring" \
    org.opencontainers.image.version="${ALLOY_VERSION}" \
    org.opencontainers.image.licenses="Apache-2.0" \
    org.opencontainers.image.vendor="Materialize, Inc." \
    org.opencontainers.image.title="Materialize Monitoring Alloy" \
    org.opencontainers.image.documentation="https://materializeinc.github.io/materialize-monitoring/reference/images/alloy/" \
    org.opencontainers.image.description="Distroless Grafana Alloy for Materialize Monitoring" \
    org.opencontainers.image.base.name="gcr.io/distroless/base-debian13" \
    cloud.materialize.image.upstream="https://github.com/grafana/alloy"

COPY --from=deb-support /usr/local/bin/alloy /bin/alloy
COPY --from=deb-support /usr/lib/${LINUX_TAG}/libsystemd.so.0* /usr/lib/${LINUX_TAG}/
COPY --from=deb-support /etc/passwd /etc/group /etc/
COPY --chown="${UID}:${UID}" --from=deb-support /var/lib/alloy/data /var/lib/alloy/data

# Use a default user
# Note that host agent daemonset usually will need root privileges to monitor
# the whole system as a special exception
USER ${UID}:${UID}

# Usually should overlay this with an emptyDir
VOLUME ["/var/lib/alloy"]

# This is just a stub. We provide separate pipelines for live usage.
COPY --chown=${UID}:${UID} example-config.alloy /etc/alloy/config.alloy

ENTRYPOINT ["/bin/alloy"]
CMD ["run", "/etc/alloy/config.alloy", "--storage.path=/var/lib/alloy/data"]
