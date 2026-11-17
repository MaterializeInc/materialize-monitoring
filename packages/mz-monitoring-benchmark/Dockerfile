# Load-testing image: impersonates Materialize processes so the monitoring
# stack can be loaded without running Materialize at scale.
#
# Built from the repository root, because the crate is a workspace member:
#    make benchmark-image
#
# Not published. The image carries `gen-load` and `preview`; `fixture-gen`
# runs from a workstation, so the build leaves out its Kubernetes client.

FROM rust:1.98-slim-trixie@sha256:4cd829461bd5c4d511c32e269da9cb8929223b666519d8004e35fc8d1d771ab7 AS build

WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p mz-monitoring-benchmark --no-default-features \
    && cp target/release/mz-monitoring-benchmark /mz-monitoring-benchmark

RUN /mz-monitoring-benchmark preview --role environmentd

FROM gcr.io/distroless/cc-debian13@sha256:809d04afa1be033c441f76bb0051878c342fb76d0ea236bc6dc94b655fa0fbd6

LABEL org.opencontainers.image.source="https://github.com/MaterializeInc/materialize-monitoring" \
    org.opencontainers.image.licenses="BSL-1.1" \
    org.opencontainers.image.vendor="Materialize, Inc." \
    org.opencontainers.image.title="Materialize Monitoring Benchmark" \
    org.opencontainers.image.description="Synthetic Materialize metrics and logs for load-testing the monitoring stack" \
    org.opencontainers.image.base.name="gcr.io/distroless/cc-debian13"

COPY --from=build /mz-monitoring-benchmark /usr/local/bin/mz-monitoring-benchmark

USER 65532:65532
EXPOSE 6878
ENTRYPOINT ["/usr/local/bin/mz-monitoring-benchmark"]
CMD ["gen-load", "--help"]
