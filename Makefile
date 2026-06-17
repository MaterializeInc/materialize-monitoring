
### MAKE FEATURES ###

# SECONDEXPANSION allows us to use $$ variables in prerequisites
.SECONDEXPANSION:

### SETUP ###
# Configuration specific to this project

# make with no target invokes this (FIXME: binaries is a placeholder for now)
.DEFAULT_GOAL := all

# Rust targets
ALL_BINARIES = mz-monitoring-build mz-monitoring-check
# Rust sources
SOURCES_mzmon-lib = $(shell find packages/mzmon-lib -type f)
SOURCES_mz-monitoring-build = $(shell find packages/mz-monitoring-build -type f)
SOURCES_mz-monitoring-check = $(shell find packages/mz-monitoring-check -type f)

# Alloy targets
ALLOY_TARGETS = gateway agent

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

# Invoke helm-docs as a tool (set HELM_DOCS=helm-docs to use brew)
HELM_DOCS ?= $(GO) tool helm-docs

# Whether brew can be used for installs (use ifneq)
HAS_BREW := $(shell command -v brew 2> /dev/null)

### PHONY TARGETS ###
# These are pseudo goals that may be easily invoked by the end user

# Build all project binaries
binaries: $(addprefix target/debug/,$(ALL_BINARIES))
.PHONY: binaries

# Build all Helm charts
charts: materialize-monitoring-chart
.PHONY: charts

docs: docs/public
.PHONY: docs

helm-docs: \
	charts/materialize-monitoring/README.md \
	docs/content/reference/helm/materialize-monitoring-values.md
.PHONY: helm-docs

# Generate grafana dashboards
grafana-dashboards: charts/materialize-monitoring/pre-rendered/dashboards/grafana docs/assets/dashboards/grafana
.PHONY: grafana-dashboards

alloy-pipelines: charts/materialize-monitoring/pre-rendered/pipelines
.PHONY: alloy-pipelines

# Make all dashboards
dashboards: grafana-dashboards
.PHONY: dashboards

pipelines: alloy-pipelines
.PHONY: pipelines

prometheus-scrapers: charts/materialize-monitoring/pre-rendered/scrapers docs/assets/prometheus-scrapers
.PHONY: prometheus-scrapers

scrapers: prometheus-scrapers
.PHONY: scrapers

synced: dashboards charts pipelines scrapers
.PHONY: synced

all: synced
.PHONY: all

### REPO MAINTENANCE ###

check-lfs:
	./bin/check-lfs.sh
.PHONY: check-lfs

### RUST TOOLING ###
# Rust binary name
BUILD_BIN_BASENAME = $(notdir $@)

target/debug/mz-monitoring-%: $$(SOURCES_mz-monitoring-%) $(SOURCES_mzmon-lib)
	cargo build --bin "$(BUILD_BIN_BASENAME)"
	# Ensure target uses a newer timestamp (cargo build can leave this as old)
	touch "$@"

## YAGNI
# target/release/mz-monitoring-%: $$(SOURCES_mz-monitoring-%) $(SOURCES_mzmon-lib)
# 	cargo build --release --bin "$(BUILD_BIN_BASENAME)"

### DASHBOARD SYNC ###

SOURCES_py-mzmon-lib = $(shell find packages/py-mzmon-lib/src -type f)
SOURCES_grafana-dashboards = $(shell find packages/grafana-dashboards/dashboards -type f) $(SOURCES_py-mzmon-lib)

charts/materialize-monitoring/pre-rendered/dashboards/grafana: $(SOURCES_grafana-dashboards)
	mkdir -p "$@"
	rm -f "$@/"*.yaml
	$(PY_RUN) -m dashboards.render -o "$@" --format yaml
	touch "$@"

### PIPELINE SYNC ###

ALLOY_TARGET = $(patsubst %.alloy,%,$(notdir $@))

# TODO: invoke
charts/materialize-monitoring/pre-rendered/pipelines/%.alloy: packages/alloy-pipelines/%.yaml target/debug/mz-monitoring-build
	mkdir -p "$(@D)"
	target/debug/mz-monitoring-build gen-pipelines --output-dir "$(@D)" --target "$(ALLOY_TARGET)"
	alloy validate "$@"

charts/materialize-monitoring/pre-rendered/pipelines: $(addprefix charts/materialize-monitoring/pre-rendered/pipelines/,$(addsuffix .alloy,$(ALLOY_TARGETS)))
	touch "$@"

### SCRAPER SYNC ###

# defensive check for typo'd extensions
_BAD_SCRAPER_NAMES = $(wildcard packages/prometheus-scrapers/*.json packages/prometheus-scrapers/*.kyaml packages/prometheus-scrapers/*.yml)
ifneq ($(_BAD_SCRAPER_NAMES),)
$(error "Unexpected scraper files with non-.yaml extensions: $(_BAD_SCRAPER_NAMES)")
endif

# Render the prometheus-operator Monitors into every consumer format, prefixed by
# format: classic- (combined classic scrape_configs), prometheus-operator- (one
# per monitor; today a validated passthrough), and gmp- (one PodMonitoring per
# PodMonitor). Replaces the old per-file cp. Clear stale outputs first so renamed
# or removed monitors don't leave orphans behind.
charts/materialize-monitoring/pre-rendered/scrapers: $(wildcard packages/prometheus-scrapers/*.yaml) target/debug/mz-monitoring-build
	mkdir -p "$@"
	rm -f "$@/"*.yaml
	target/debug/mz-monitoring-build gen-scrape-configs \
		--format classic --format prometheus-operator --format gmp \
		--output-dir "$@"
	touch "$@"

docs/assets/prometheus-scrapers: charts/materialize-monitoring/pre-rendered/scrapers
	mkdir -p "$@"
	rm -f "$@/"*.yaml
	cp charts/materialize-monitoring/pre-rendered/scrapers/*.yaml "$@"
	touch "$@"

# Re-extract the prometheus-operator CRD JSONSchemas from the vendored
# materialize-monitoring-crds chart. Output is checked in; re-run on version bump.
crd-schemas:
	bin/extract-crd-schemas.sh
.PHONY: crd-schemas

### HELM CHARTS ###

# Helm chart name
CHART_NAME = $(dir $(patsubst charts/%,%,$@))

# Shared sources that drive helm-docs regeneration
HELM_DOCS_SOURCES_materialize-monitoring = \
	charts/materialize-monitoring/values.yaml \
	charts/materialize-monitoring/Chart.yaml

charts/materialize-monitoring/pre-rendered: charts/materialize-monitoring/pre-rendered/dashboards/grafana charts/materialize-monitoring/pre-rendered/pipelines charts/materialize-monitoring/pre-rendered/scrapers
	touch "$@"

# Generate the chart-local README.md from values.yaml + the README template.
charts/materialize-monitoring/README.md: \
		$(HELM_DOCS_SOURCES_materialize-monitoring) \
		tools/chartlib/helm-docs-lib.gotmpl \
		charts/materialize-monitoring/README.md.gotmpl
	$(HELM_DOCS) \
		--chart-search-root charts/materialize-monitoring \
		--template-files ../../tools/chartlib/helm-docs-lib.gotmpl \
		--template-files README.md.gotmpl \
		--output-file README.md \
		--sort-values-order file \
		--log-level debug \
		--ignore-non-descriptions

# Generate the docsite values reference from the same values.yaml.
# Output and template paths are relative to the chart directory, hence
# the `../../` prefix.
docs/content/reference/helm/materialize-monitoring-values.md: \
		$(HELM_DOCS_SOURCES_materialize-monitoring) \
		tools/chartlib/helm-docs-lib.gotmpl \
		docs/content/reference/helm/materialize-monitoring-values.md.gotmpl
	$(HELM_DOCS) \
		--chart-search-root charts/materialize-monitoring \
		--template-files ../../tools/chartlib/helm-docs-lib.gotmpl \
		--template-files ../../docs/content/reference/helm/materialize-monitoring-values.md.gotmpl \
		--output-file ../../docs/content/reference/helm/materialize-monitoring-values.md \
		--sort-values-order file \
		--log-level debug \
		--ignore-non-descriptions

# Do any necessary generation for this chart
charts/materialize-monitoring: charts/materialize-monitoring/README.md charts/materialize-monitoring/pre-rendered
	touch "$@"

HELM_VERSION_materialize-monitoring = $(shell yq e '.version' charts/materialize-monitoring/Chart.yaml)
charts/materialize-monitoring-$(HELM_VERSION_materialize-monitoring).tgz: charts/materialize-monitoring
	helm package charts/materialize-monitoring --destination charts/
	test -f "$@"

materialize-monitoring-chart: charts/materialize-monitoring-$(HELM_VERSION_materialize-monitoring).tgz
.PHONY: materialize-monitoring-chart

### HUGO DOCS ###

serve-docs:
	$(HUGO_BIN) --source docs serve --gc --buildDrafts --openBrowser
.PHONY: serve-docs

docs/assets/dashboards/grafana: $(SOURCES_grafana-dashboards)
	mkdir -p "$@"
	rm -f "$@/"*.json
	$(PY_RUN) -m dashboards.render -o "$@" --format json
	touch "$@"

# Generate docs
docs/public: \
		$(shell find docs/content) \
		docs/content/reference/helm/materialize-monitoring-values.md
	$(HUGO_BIN) --source docs --destination public
	touch "$@"
