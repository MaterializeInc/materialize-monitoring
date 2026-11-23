# Load-testing image: impersonates Materialize processes so the monitoring
# stack can be loaded without running Materialize at scale.
#
# Built from the repository root, because the crate is a workspace member:
#    make benchmark-image
# Released by the benchmark-images component (.github/actions/build-benchmark-image).
#
# The image carries `gen-load` and `preview`; `fixture-gen` runs from a
# workstation, so the build leaves out its Kubernetes client.

# The build runs on the builder's own platform and cross-compiles, so a
# multi-arch build never compiles Rust under emulation. Without the Kubernetes
# client the crate is pure Rust, so a cross linker is all a target needs.
FROM --platform=$BUILDPLATFORM rust:1.98-slim-trixie@sha256:4cd829461bd5c4d511c32e269da9cb8929223b666519d8004e35fc8d1d771ab7 AS build

ARG BUILDARCH
ARG TARGETARCH
RUN set -eu; \
    case "${TARGETARCH}" in \
        amd64) triple=x86_64-unknown-linux-gnu; gcc=gcc-x86-64-linux-gnu; linker=x86_64-linux-gnu-gcc ;; \
        arm64) triple=aarch64-unknown-linux-gnu; gcc=gcc-aarch64-linux-gnu; linker=aarch64-linux-gnu-gcc ;; \
        *) echo "unsupported TARGETARCH ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    if [ "${TARGETARCH}" = "${BUILDARCH}" ]; then \
        linker=cc; \
    else \
        export DEBIAN_FRONTEND=noninteractive; \
        apt-get update; \
        apt-get install -qy --no-install-recommends "${gcc}" "libc6-dev-${TARGETARCH}-cross"; \
        rm -rf /var/lib/apt/lists/*; \
    fi; \
    rustup target add "${triple}"; \
    echo "${triple}" > /triple; \
    echo "${linker}" > /linker

WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target,id=benchmark-target-${TARGETARCH} \
    set -eu; \
    triple="$(cat /triple)"; \
    linker="$(cat /linker)"; \
    env_triple="$(echo "${triple}" | tr 'a-z-' 'A-Z_')"; \
    export "CARGO_TARGET_${env_triple}_LINKER=${linker}"; \
    cargo build --release --locked -p mz-monitoring-benchmark --no-default-features --target "${triple}"; \
    cp "target/${triple}/release/mz-monitoring-benchmark" /mz-monitoring-benchmark

FROM gcr.io/distroless/cc-debian13@sha256:dac90a25ffe03ebaace90cda886b081e61ab6cd77ac47eda51f7bcc15c320765

LABEL org.opencontainers.image.source="https://github.com/MaterializeInc/materialize-monitoring" \
    org.opencontainers.image.licenses="BSL-1.1" \
    org.opencontainers.image.vendor="Materialize, Inc." \
    org.opencontainers.image.title="Materialize Monitoring Benchmark" \
    org.opencontainers.image.description="Synthetic Materialize metrics and logs for load-testing the monitoring stack" \
    org.opencontainers.image.base.name="gcr.io/distroless/cc-debian13"

COPY --from=build /mz-monitoring-benchmark /usr/local/bin/mz-monitoring-benchmark

# Smoke-test the binary on the image it ships in, for each platform: it loads
# its embedded fixtures and renders a scrape. Exec form, because distroless has
# no shell; a non-native platform runs under QEMU, which is cheap at this size.
RUN ["/usr/local/bin/mz-monitoring-benchmark", "preview", "--role", "environmentd"]

USER 65532:65532
EXPOSE 6878
ENTRYPOINT ["/usr/local/bin/mz-monitoring-benchmark"]
CMD ["gen-load", "--help"]
