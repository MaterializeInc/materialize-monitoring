# Minimal distroless image for running alloy in materialize-monitoring.
# This is an alternative to https://github.com/grafana/alloy/blob/main/Dockerfile
# To build this, run:
#    make alloy-image
# This uses the FIPS 140-3 compliant alloy-boringcrypto binary
# This can be used as a drop-in replacement for the upstream alloy image

## Supporting libraries
FROM debian:13@sha256:34cd9e9fd437c0a095ec39cb2e73422c9f30821b0d0848ed74fd0d43bae4d958 AS deb-support

# Standard alloy uid/username
ARG UID="473"
ARG USERNAME="alloy"

# renovate: datasource=github-releases packageName=grafana/alloy
ARG ALLOY_VERSION=v1.18.1

# Seed with distroless user/group files
COPY --from=gcr.io/distroless/base-debian13@sha256:f4a335ca209e1d2ee873102c17c389ad0142e3d5b21aee2817e9cc9c01d87d20 /etc/passwd /etc/group /etc/

# Create alloy user in container, but do not set it as default
RUN groupadd --gid "$UID" "$USERNAME" \
    && useradd -r -s /usr/sbin/nologin -u "$UID" -g "$UID" "$USERNAME" \
    && mkdir -p /var/lib/alloy/data \
    && chown -R "$USERNAME:$USERNAME" /var/lib/alloy \
    && chmod -R 770 /var/lib/alloy

ARG TARGETARCH
ARG TARGETOS
ARG LINUX_TAG1=${TARGETARCH/arm64/aarch64}
ARG LINUX_TAG=${LINUX_TAG1/amd64/x86_64}-linux-gnu

# binutils is here for `objdump` in the staging assertion below. It stays in this
# stage; nothing from it reaches the final image.
#
# The compression libraries are not libsystemd's linked dependencies — see the
# staging step for why they are installed anyway.
RUN export DEBIAN_FRONTEND="noninteractive" \
    && apt-get update \
    && apt-get upgrade -y \
    && apt-get install -qy --no-install-recommends \
        libsystemd0 \
        libgcrypt20 \
        liblz4-1 \
        liblzma5 \
        binutils \
        unzip \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# What the distroless base already ships. Read out of the image rather than
# hardcoded, so a base bump cannot quietly turn "the base provides this" into a
# library that is missing at runtime.
COPY --from=gcr.io/distroless/base-debian13@sha256:f4a335ca209e1d2ee873102c17c389ad0142e3d5b21aee2817e9cc9c01d87d20 /usr/lib/${LINUX_TAG}/*.so* /base-lib/

# Stage libsystemd and everything it needs, by either mechanism, for the final
# image.
#
# Alloy's `loki.source.journal` dlopens libsystemd at runtime rather than linking
# it, so a gap here breaks nothing the build or the container start can observe.
# The component just reports unhealthy forever — "unable to open a handle to the
# library" — and journal collection silently never runs, which is what happened
# while this copied `libsystemd.so.0` alone (DEP-230).
#
# Two kinds of dependency, and only one of them is derivable:
#
#   1. **Linked** (`DT_NEEDED`). Today that is `libcap.so.2` plus glibc, and
#      `libcap` is the one whose absence actually fails the dlopen. Resolved with
#      `ldd` rather than listed, so a Debian bump that adds one is picked up on
#      the next build instead of becoming the same silent breakage.
#
#   2. **dlopen'd by libsystemd itself** — the journal compressors. journald
#      compresses entries (zstd by default, lzma and lz4 on older or
#      differently-configured hosts) and libgcrypt covers sealed journals. These
#      appear nowhere in the ELF headers: libsystemd opens them by name at
#      runtime, so no amount of dependency resolution can find them and they have
#      to be named. Without them libsystemd loads and journal *reads* fail on any
#      compressed entry.
#
# Seeding the closure with both means the compressors' own linked dependencies —
# `libgpg-error.so.0` behind libgcrypt, for instance — still resolve automatically.
RUN set -eu; \
    mkdir -p "/staging/usr/lib/${LINUX_TAG}"; \
    # Keep this list next to the reason it cannot be derived. `libzstd.so.1` is
    # in it for completeness even though the distroless base already ships it —
    # the skip below handles that, and an entry that stops being redundant should
    # not have to be noticed first.
    dlopened="libgcrypt.so.20 liblz4.so.1 liblzma.so.5 libzstd.so.1"; \
    seeds="/usr/lib/${LINUX_TAG}/libsystemd.so.0"; \
    for name in ${dlopened}; do \
        path="/usr/lib/${LINUX_TAG}/${name}"; \
        # A named library that is neither installed nor in the base is a broken
        # assumption, not something to skip past.
        if [ ! -e "${path}" ] && [ ! -e "/base-lib/${name}" ]; then \
            echo "ERROR: ${name} is named as dlopen'd but is not installed" >&2; \
            exit 1; \
        fi; \
        if [ -e "${path}" ]; then seeds="${seeds} ${path}"; fi; \
    done; \
    { for s in ${seeds}; do echo "$s"; ldd "$s" | awk '/=> \//{print $3}'; done; } \
      | sort -u >/tmp/closure; \
    while read -r so; do \
        name="$(basename "$so")"; \
        # Skip anything the base has: replacing a base library — glibc above all
        # — with Debian's build of it is the substitution distroless exists to
        # avoid, and none of these need it.
        if [ -e "/base-lib/${name}" ]; then continue; fi; \
        # -L so each lands as a real file named by its SONAME, which is what the
        # loader searches for. The symlink chain is not needed.
        cp -L "$so" "/staging/usr/lib/${LINUX_TAG}/${name}"; \
    done </tmp/closure

# Fail the build if a staged library still needs something that will not exist in
# the final image. This is the guard that makes the resolution above trustworthy:
# without it, a silent gap here is only observable as an unhealthy component on a
# running cluster, which is how DEP-230 went unnoticed.
RUN set -eu; \
    have="$( { ls -1 "/staging/usr/lib/${LINUX_TAG}"; ls -1 /base-lib; } | tr '\n' ' ')"; \
    missing=""; \
    for so in "/staging/usr/lib/${LINUX_TAG}"/*; do \
        for need in $(objdump -p "$so" | awk '/NEEDED/ {print $2}'); do \
            case " ${have} " in \
                *" ${need} "*) ;; \
                *) missing="${missing} ${need}";; \
            esac; \
        done; \
    done; \
    if [ -n "${missing}" ]; then \
        echo "ERROR: staged libraries still need:${missing}" >&2; \
        echo "Neither /staging nor the distroless base provides them." >&2; \
        exit 1; \
    fi; \
    echo "staged for the final image:"; ls -1 "/staging/usr/lib/${LINUX_TAG}"

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
FROM gcr.io/distroless/base-debian13@sha256:f4a335ca209e1d2ee873102c17c389ad0142e3d5b21aee2817e9cc9c01d87d20

ARG TARGETARCH
ARG LINUX_TAG1=${TARGETARCH/arm64/aarch64}
ARG LINUX_TAG=${LINUX_TAG1/amd64/x86_64}-linux-gnu

# renovate: datasource=github-releases packageName=grafana/alloy
ARG ALLOY_VERSION=v1.18.1
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
# libsystemd plus its resolved dependency closure — see the staging step above.
COPY --from=deb-support /staging/usr/lib/${LINUX_TAG}/ /usr/lib/${LINUX_TAG}/
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
