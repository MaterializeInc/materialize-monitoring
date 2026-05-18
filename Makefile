
### MAKE FEATURES ###

# SECONDEXPANSION allows us to use $$ variables in prerequisites
.SECONDEXPANSION:

### SETUP ###
# Configuration specific to this project

# make with no target invokes this (FIXME: binaries is a placeholder for now)
.DEFAULT_GOAL := binaries

# Helm chart names
CHARTS = materialize-monitoring

# Rust targets
ALL_BINARIES = mz-monitoring-build mz-monitoring-check
# Rust sources
SOURCES_mz-monitoring-build = $(shell find crates/mz-monitoring-build -type f)
SOURCES_mz-monitoring-check = $(shell find crates/mz-monitoring-check -type f)

### CONFIG ###
# These may be overridden by the user

# Go binary (can provide an alternative path to a compatible binary)
GO ?= go

# Prefix for all python commands
# TODO: detect other cases
PY_RUN := uv run

# Invoke hugo as a tool (you can use HUGO_BIN=hugo to use brew)
# By default, we get the one from go.mod
HUGO_BIN ?= GOFLAGS=-tags=extended $(GO) tool hugo

# Whether brew can be used for installs (use ifneq)
HAS_BREW := $(shell command -v brew 2> /dev/null)

### PHONY TARGETS ###
# These are pseudo goals that may be easily invoked by the end user

# Build all project binaries
binaries: $(addprefix target/debug/,$(ALL_BINARIES))
.PHONY: binaries

# Build all Helm charts
charts: $(addprefix charts/,$(CHARTS))
.PHONY: charts

docs: docs/public
.PHONY: docs

grafana-dashboards: charts/materialize-monitoring/pre-rendered/dashboards/grafana
.PHONY: grafana-dashboards

dashboards: grafana-dashboards
.PHONY: dashboards

synced: dashboards charts
.PHONY: synced

### REPO MAINTENANCE ###

check-lfs:
	./bin/check-lfs.sh
.PHONY: check-lfs

### RUST TOOLING ###
# Rust binary name
BUILD_BIN_BASENAME = $(notdir $@)

target/debug/mz-monitoring-%: $$(SOURCES_mz-monitoring-%)
	cargo build --bin "$(BUILD_BIN_BASENAME)"

## YAGNI
# target/release/mz-monitoring-%: $$(SOURCES_mz-monitoring-%)
# 	cargo build --release --bin "$(BUILD_BIN_BASENAME)"

### DASHBOARD SYNC ###

SOURCES_grafana-dashboards = $(shell find packages/grafana-dashboards/dashboards -type f)

charts/materialize-monitoring/pre-rendered/dashboards/grafana: $(SOURCES_grafana-dashboards)
	rm -f "$@/"*.json
	$(PY_RUN) -m dashboards.render -o "$@" --format json
	touch "$@"

### HELM CHARTS ###

# Helm chart name
CHART_NAME = $(dir $(patsubst charts/%,%,$@))

# Update helm chart README
# WARNING: if README.md is updated after values.yaml, this won't run
charts/materialize-monitoring/README.md: charts/materialize-monitoring/values.yaml
	bin/helm-readme-sync "$(CHART_NAME)"
	touch "$@"

# TODO: helm package
HELM_VERSION_materialize-monitoring = TODO.0.0
charts/materialize-monitoring-$(HELM_VERSION_materialize-monitoring).tgz: charts/materialize-monitoring/README.md
	helm package charts/materialize-monitoring --destination charts/
	test -f "$@"

# Do any necessary generation for this chart
charts/materialize-monitoring: charts/materialize-monitoring-$(HELM_VERSION_materialize-monitoring).tgz
	touch "$@"


### HUGO DOCS ###

serve-docs:
	$(HUGO_BIN) --source docs serve --gc --buildDrafts --openBrowser
.PHONY: serve-docs

# Generate docs
docs/public: $(shell find docs/content -type f)
	$(HUGO_BIN) --source docs --destination public
	touch "$@"
