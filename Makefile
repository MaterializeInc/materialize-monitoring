
### MAKE FEATURES ###

# SECONDEXPANSION allows us to use $$ variables in prerequisites
.SECONDEXPANSION:

### SETUP ###
# Configuration specific to this project

# make with no target invokes this (FIXME: binaries is a placeholder for now)
.DEFAULT_GOAL := all

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
charts: $(addprefix charts/,$(CHARTS))
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

# Make all dashboards
dashboards: grafana-dashboards
.PHONY: dashboards

synced: dashboards charts
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

target/debug/mz-monitoring-%: $$(SOURCES_mz-monitoring-%)
	cargo build --bin "$(BUILD_BIN_BASENAME)"

## YAGNI
# target/release/mz-monitoring-%: $$(SOURCES_mz-monitoring-%)
# 	cargo build --release --bin "$(BUILD_BIN_BASENAME)"

### DASHBOARD SYNC ###

SOURCES_py-mzmon-lib = $(shell find packages/py-mzmon-lib/src -type f)
SOURCES_grafana-dashboards = $(shell find packages/grafana-dashboards/dashboards -type f) $(SOURCES_py-mzmon-lib)

charts/materialize-monitoring/pre-rendered/dashboards/grafana: $(SOURCES_grafana-dashboards)
	mkdir -p "$@"
	rm -f "$@/"*.yaml
	$(PY_RUN) -m dashboards.render -o "$@" --format yaml
	touch "$@"

### HELM CHARTS ###

# Helm chart name
CHART_NAME = $(dir $(patsubst charts/%,%,$@))

# Shared sources that drive helm-docs regeneration
HELM_DOCS_SOURCES_materialize-monitoring = \
	charts/materialize-monitoring/values.yaml \
	charts/materialize-monitoring/Chart.yaml

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

HELM_VERSION_materialize-monitoring = $(shell yq e '.version' charts/materialize-monitoring/Chart.yaml)
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
