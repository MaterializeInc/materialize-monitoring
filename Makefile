
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
SOURCES_mz-monitoring-e2e = $(shell find packages/mz-monitoring-e2e -type f)

# Alloy targets
ALLOY_TARGETS = gateway gateway-metrics gateway-dest-stub agent

### CONFIG ###
# These may be overridden by the user

# Go binary (can provide an alternative path to a compatible binary)
GO ?= go

# Prefix for all python commands
# TODO: detect other cases
PY_RUN := uv run

# Invoke hugo as a tool (you can use HUGO_BIN=hugo to use brew)
# By default, we get the one from go.mod.
# This is `go run`, not `go tool`, because the book theme needs the extended
# edition (libsass) and `go tool` silently ignores `-tags` — it only accepts
# -C, -overlay, -modcacherw, and -modfile.
HUGO_BIN ?= $(GO) run -tags extended github.com/gohugoio/hugo

# Invoke helm-docs as a tool (set HELM_DOCS=helm-docs to use brew)
HELM_DOCS ?= $(GO) tool helm-docs

# Terraform tooling. Not pinned via go.mod (neither publishes a Go tool
# module), so these expect the binaries on PATH.
TERRAFORM ?= terraform
TERRAFORM_DOCS ?= terraform-docs

# Whether brew can be used for installs (use ifneq)
HAS_BREW := $(shell command -v brew 2> /dev/null)

### PHONY TARGETS ###
# These are pseudo goals that may be easily invoked by the end user

# Build all project binaries
binaries: $(addprefix target/debug/,$(ALL_BINARIES))
.PHONY: binaries

# Build all docker images
docker-images: alloy-image
.PHONY: docker-images

# Build all Helm charts
charts: materialize-monitoring-chart
.PHONY: charts

docs: docs/public
.PHONY: docs

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

metrics: metric-tiers docs/assets/metrics/metrics.yaml
.PHONY: metrics

metric-tiers: charts/materialize-monitoring/pre-rendered/metrics/metric-tiers.yaml
.PHONY: metric-tiers

synced: dashboards charts pipelines scrapers metric-tiers
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

# Render each target. Validation happens in the aggregate target below, because
# gateway.alloy is not a standalone config (it references loki.process.egress,
# supplied by gateway-dest-stub.alloy) and must be validated joined with it.
charts/materialize-monitoring/pre-rendered/pipelines/%.alloy: packages/alloy-pipelines/%.yaml target/debug/mz-monitoring-build
	mkdir -p "$(@D)"
	target/debug/mz-monitoring-build gen-pipelines --output-dir "$(@D)" --target "$(ALLOY_TARGET)"

charts/materialize-monitoring/pre-rendered/pipelines: $(addprefix charts/materialize-monitoring/pre-rendered/pipelines/,$(addsuffix .alloy,$(ALLOY_TARGETS)))
	$(MAKE) alloy-pipelines-validate
	touch "$@"

# Validate rendered pipelines. agent is currently a self-contained config; the
# gateway is not — gateway.alloy forwards to loki.process.egress, defined in the
# destination stub — so validate the two together the way alloy loads a config
# directory.
PIPELINES_DIR = charts/materialize-monitoring/pre-rendered/pipelines
alloy-pipelines-validate:
	alloy validate "$(PIPELINES_DIR)/agent.alloy"
	cat "$(PIPELINES_DIR)/gateway.alloy" "$(PIPELINES_DIR)/gateway-metrics.alloy" "$(PIPELINES_DIR)/gateway-dest-stub.alloy" | alloy validate /dev/stdin
.PHONY: alloy-pipelines-validate

### SCRAPER SYNC ###

# defensive check for typo'd extensions
_BAD_SCRAPER_NAMES = $(wildcard packages/prometheus-scrapers/*.json packages/prometheus-scrapers/*.kyaml packages/prometheus-scrapers/*.yml)
ifneq ($(_BAD_SCRAPER_NAMES),)
$(error "Unexpected scraper files with non-.yaml extensions: $(_BAD_SCRAPER_NAMES)")
endif

SCRAPER_FORMATS = classic prometheus-operator gmp

# Render the prometheus-operator Monitors into every consumer format, prefixed by
# format: classic/ (combined classic scrape_configs), prometheus-operator/ (one
# per monitor; today a validated passthrough), and gmp/ (one PodMonitoring per
# PodMonitor). Clear stale outputs first so renamed
# or removed monitors don't leave orphans behind.
charts/materialize-monitoring/pre-rendered/scrapers: $(wildcard packages/prometheus-scrapers/*.yaml) target/debug/mz-monitoring-build
	mkdir -p "$@"
	rm -f "$@/"*/*.yaml
	target/debug/mz-monitoring-build gen-scrape-configs \
		$(foreach format,$(SCRAPER_FORMATS),--format $(format)) \
		--output-dir "$@"
	touch "$@"

docs/assets/prometheus-scrapers: charts/materialize-monitoring/pre-rendered/scrapers
	rm -rf "$@/"
	mkdir -p "$@"
	cp -r charts/materialize-monitoring/pre-rendered/scrapers/* "$@"
	touch "$@"

# Group the query registry's metrics by importance for the gateway's
# per-destination allowlists (charts consume this via $.Files).
charts/materialize-monitoring/pre-rendered/metrics/metric-tiers.yaml: $(wildcard packages/queries/*.yaml) target/debug/mz-monitoring-build
	mkdir -p "$(@D)"
	target/debug/mz-monitoring-build gen-metric-tiers \
		--source-dir packages/queries \
		--out "$@"

docs/assets/metrics/metrics.yaml: $(wildcard packages/queries/*.yaml) target/debug/mz-monitoring-build
	mkdir -p "$(@D)"
	target/debug/mz-monitoring-build extract-metrics --out-dir docs/assets/metrics/

# Re-extract the prometheus-operator CRD JSONSchemas from the vendored
# materialize-monitoring-crds chart. Output is checked in; re-run on version bump.
crd-schemas:
	bin/extract-crd-schemas.sh
.PHONY: crd-schemas

# Re-deflate the Grafana Operator CRDs from the vendored grafana-operator chart
# into materialize-monitoring-crds. Output is checked in; re-run on version bump.
grafana-operator-crds:
	bin/extract-grafana-operator-crds.sh
.PHONY: grafana-operator-crds

### CONTAINER IMAGES ###

CONTAINER_REGISTRY ?= ghcr.io/materializeinc

# Upstream alloy version is used for the tag
ALLOY_VERSION ?= $(shell grep -E '^ARG ALLOY_VERSION=' packages/alloy/Dockerfile | head -n1 | cut -d= -f2)
# Extra suffix if there are multiple images at the same version (revert back to mz1 on upgrade)
ALLOY_SUFFIX ?= mz2

alloy-image.iid: $(wildcard packages/alloy/*)
	docker buildx build --load --platform linux/amd64,linux/arm64 --iidfile "$@" --tag $(CONTAINER_REGISTRY)/mzmon-alloy:$(ALLOY_VERSION)-$(ALLOY_SUFFIX) packages/alloy/
	docker run --platform linux/amd64 --rm $$(cat "$@") --version
	docker run --platform linux/arm64 --rm $$(cat "$@") --version

alloy-image: alloy-image.iid
.PHONY: alloy-image

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

HELM_DOCS_SOURCES_materialize-monitoring-crds = \
	charts/materialize-monitoring-crds/values.yaml \
	charts/materialize-monitoring-crds/Chart.yaml

# Generate the chart-local README.md from values.yaml + the README template.
# --chart-search-root is the chart itself so helm-docs does not wander into
# the vendored grafana-operator-crds subchart and write a README there too.
charts/materialize-monitoring-crds/README.md: \
		$(HELM_DOCS_SOURCES_materialize-monitoring-crds) \
		tools/chartlib/helm-docs-lib.gotmpl \
		charts/materialize-monitoring-crds/README.md.gotmpl
	$(HELM_DOCS) \
		--chart-search-root charts/materialize-monitoring-crds \
		--template-files ../../tools/chartlib/helm-docs-lib.gotmpl \
		--template-files README.md.gotmpl \
		--output-file README.md \
		--sort-values-order file \
		--log-level debug \
		--ignore-non-descriptions

HELM_VERSION_materialize-monitoring = $(shell yq e '.version' charts/materialize-monitoring/Chart.yaml)
charts/materialize-monitoring-$(HELM_VERSION_materialize-monitoring).tgz: charts/materialize-monitoring
	helm package charts/materialize-monitoring --destination charts/
	test -f "$@"

materialize-monitoring-chart: charts/materialize-monitoring-$(HELM_VERSION_materialize-monitoring).tgz
.PHONY: materialize-monitoring-chart

# Charts whose charts/ directory holds committed subchart archives.
HELM_DEP_CHARTS = \
	charts/materialize-monitoring \
	charts/materialize-monitoring-crds

# Populate charts/*/charts/ from Chart.lock and drop archives that no longer
# match. The .tgz files are committed (through LFS, see .gitattributes), so this
# has to run whenever Chart.yaml or Chart.lock moves or the checked-in archives
# go stale against the lock.
#
# `build` is preferred: it installs exactly the locked versions, so a bump PR
# gets the version its lock names rather than whatever is newest in the range
# at CI time. It refuses to run when Chart.lock has drifted from Chart.yaml,
# which is what a PR that edits only the version range looks like — fall back to
# `update` there, which re-resolves the range and rewrites the lock.
helm-deps:
	@for c in $(HELM_DEP_CHARTS); do \
		echo "==> helm dependency build $$c"; \
		helm dependency build "$$c" || { \
			echo "==> Chart.lock out of sync with Chart.yaml; falling back to update"; \
			helm dependency update "$$c"; \
		}; \
	done
.PHONY: helm-deps

HELM_UNITTEST_ARGS ?=

helm-tests:
	helm unittest $(HELM_UNITTEST_ARGS) charts/materialize-monitoring
	helm unittest $(HELM_UNITTEST_ARGS) charts/materialize-monitoring-crds
.PHONY: helm-tests

helm-update-snapshots:
	$(MAKE) helm-tests HELM_UNITTEST_ARGS="--update-snapshot"
.PHONY: helm-update-snapshots

helm-docs: \
	charts/materialize-monitoring/README.md \
	charts/materialize-monitoring-crds/README.md \
	docs/content/reference/helm/materialize-monitoring-values.md
.PHONY: helm-docs

# Regenerate the Terraform module READMEs in place (inject between the
# BEGIN_TF_DOCS/END_TF_DOCS markers). Not pinned via go.mod like helm-docs —
# install terraform-docs separately.
terraform-docs: \
	docs/content/reference/terraform/materialize-monitoring-variables.md
	@for m in terraform/modules/*/; do \
		echo "terraform-docs $$m"; \
		$(TERRAFORM_DOCS) -c .terraform-docs.yml "$$m" >/dev/null; \
	done
.PHONY: terraform-docs

# Generate the docsite variable reference from the same variables.tf. The
# output path in the config is relative to the module directory, hence the
# `../../../` prefix there.
docs/content/reference/terraform/materialize-monitoring-variables.md: \
		terraform/modules/materialize-monitoring/variables.tf \
		terraform/modules/materialize-monitoring/outputs.tf \
		.terraform-docs.docsite.yml
	$(TERRAFORM_DOCS) -c .terraform-docs.docsite.yml terraform/modules/materialize-monitoring

# Format and validate every Terraform module. `validate` needs `init`, which is
# run without a backend so it stays offline apart from provider downloads.
terraform-check: terraform-render
	$(TERRAFORM) fmt -recursive -check -diff terraform/
	@for d in terraform/modules/*/ terraform/modules/*/examples/*/ terraform/test/*/; do \
		[ -f "$$d/versions.tf" ] || [ -f "$$d/main.tf" ] || continue; \
		echo "terraform validate $$d"; \
		( cd "$$d" && $(TERRAFORM) init -backend=false -input=false >/dev/null && $(TERRAFORM) validate ); \
	done
.PHONY: terraform-check

# Plan each example and render the chart against the values it composes. This is
# the only check that proves a module value reached the setting it was aimed at —
# `validate` accepts any well-formed HCL, including a path the chart never reads.
# Needs no cluster; see the script header.
terraform-render:
	./bin/terraform-render-check.sh
.PHONY: terraform-render

### E2E (kind) ###
# Tier 1 is the fast gate: the chart's own hermetic shape, no object storage.
# Tier 2 adds the generic-cloud substrate (rustfs + CNPG), which is what exercises
# the real object-storage code paths. See test/e2e/README.md.

KIND ?= kind
KIND_CLUSTER ?= mzmon-e2e

# Every command below names this context explicitly. These targets install,
# restart, and delete things, and without `--context` they would target whatever
# the current kubeconfig context happens to be — a production cluster, if that is
# what you last used. `kubectl` and `helm` both fail on an unknown context, so a
# missing cluster is an error rather than a silent redirect to the wrong one.
KIND_CONTEXT ?= kind-$(KIND_CLUSTER)
KUBECTL ?= kubectl --context $(KIND_CONTEXT)
HELM_KUBE ?= --kube-context $(KIND_CONTEXT)

# Matches the GKE minor the stack is validated against.
#
# Digest-pinned, and it earns it: kind rebuilds node images per kind release and
# reuses the tag, so a tag alone does not identify an image. `kindest/node:v1.21.14`
# has been published with six different digests. The digest is a multi-arch index
# (amd64 + arm64), so it works on both a CI runner and an Apple Silicon laptop.
#
# renovate: datasource=docker packageName=kindest/node
KIND_NODE_IMAGE ?= kindest/node:v1.34.8@sha256:02722c2dedddcfc00febf5d27fbeb9b7b2c14294c82109ff4a85d89ac9ba3256

e2e-cluster:
	$(KIND) create cluster --config test/e2e/kind-config.yaml --image $(KIND_NODE_IMAGE) --wait 120s
	# Namespaces the operator module owns in a real install; the scrapers target
	# them, and Helm refuses to install objects into a namespace that is absent.
	$(KUBECTL) create namespace monitoring --dry-run=client -o yaml | $(KUBECTL) apply -f -
	$(KUBECTL) create namespace materialize --dry-run=client -o yaml | $(KUBECTL) apply -f -
	$(KUBECTL) create namespace materialize-environment --dry-run=client -o yaml | $(KUBECTL) apply -f -
.PHONY: e2e-cluster

e2e-cluster-down:
	$(KIND) delete cluster --name $(KIND_CLUSTER)
.PHONY: e2e-cluster-down

e2e-tier1:
	helm upgrade --install mzmon-crds charts/materialize-monitoring-crds $(HELM_KUBE) \
		--namespace monitoring --wait --timeout 5m
	helm upgrade --install mzmon charts/materialize-monitoring $(HELM_KUBE) \
		--namespace monitoring --skip-crds \
		-f charts/materialize-monitoring/profiles/loki-test.values.yaml \
		-f charts/materialize-monitoring/profiles/kind-tier1.values.yaml \
		--timeout 10m
	# Alloy's config arrives through envFrom ConfigMaps, so a config change needs a
	# restart — Helm does not roll these. The Terraform path stamps a values hash
	# for exactly this; a raw `helm upgrade` has to do it by hand.
	$(KUBECTL) rollout restart -n monitoring deployment/alloy-gateway daemonset/alloy-agent
	$(KUBECTL) rollout status -n monitoring deployment/alloy-gateway --timeout 5m
.PHONY: e2e-tier1

# The assertion suite. One binary for every tier: which assertions apply is read
# from the release's own Helm values, so the tier is a property of the cluster
# rather than a flag. Overridable so a tier-3 run can point it at a real cluster:
#
#   make e2e-verify E2E_CONTEXT=my-eks-context E2E_NAMESPACE=monitoring
E2E_BIN ?= target/debug/mz-monitoring-e2e
E2E_CONTEXT ?= $(KIND_CONTEXT)
E2E_NAMESPACE ?= monitoring
E2E_RELEASE ?= mzmon
# Same directory the CI job uploads, and the collector is idempotent — so
# whichever step fails first, the artifact is there and there is only one of it.
E2E_DIAGNOSTICS_DIR ?= e2e-diagnostics-tier1

E2E_FLAGS ?=

e2e-verify: | $(E2E_BIN)
	$(E2E_BIN) --context $(E2E_CONTEXT) --namespace $(E2E_NAMESPACE) \
		--release $(E2E_RELEASE) --diagnostics-dir $(E2E_DIAGNOSTICS_DIR) $(E2E_FLAGS)
.PHONY: e2e-verify

# Assert the stack behaves, rather than just that the install exited zero.
# Kept as its own name because that is what CI and the docs call; it is
# `e2e-verify` against the tier-1 cluster.
#
# No exemptions. This carried `--allow-unhealthy loki.source.journal.node_logs`
# while the Alloy image was missing libsystemd's dependencies (DEP-230); with
# v1.18.1-mz2 and the `/run/log/journal` mount, journal collection works and the
# component-health assertion covers it for free. Resist adding another exemption
# here without a ticket and a removal condition — the value of that assertion is
# that it has none.
e2e-verify-tier1: e2e-verify
.PHONY: e2e-verify-tier1

# The tier-2 substrate on its own: object storage and Postgres, no monitoring
# stack. Provable independently, and what a tier-2 root composes with the module.
e2e-generic-cloud:
	( cd terraform/test/generic-cloud && \
		$(TERRAFORM) init -input=false >/dev/null && \
		$(TERRAFORM) apply -auto-approve -input=false \
			-var 'kube_context=$(KIND_CONTEXT)' )
.PHONY: e2e-generic-cloud

e2e-generic-cloud-down:
	( cd terraform/test/generic-cloud && \
		$(TERRAFORM) destroy -auto-approve -input=false \
			-var 'kube_context=$(KIND_CONTEXT)' )
.PHONY: e2e-generic-cloud-down

# Remove the tier-1 install so the same cluster can host tier 2.
#
# Needed because the two tiers collide rather than coexist: both name their CRDs
# release `mzmon-crds` in `monitoring`, and Helm refuses a name already in use —
# so a tier-2 apply against a tier-1 cluster fails on the first release it tries
# to create. Recreating the cluster also works and takes minutes longer.
#
# Order is load-bearing. The main release goes first because its `pre-delete`
# hook removes the Grafana custom resources; take the CRDs out from under it and
# their finalizers have no remover, leaving the CRDs wedged in Terminating.
#
# PVCs outlive their StatefulSets by design, and a tier-2 Loki that adopts a
# tier-1 filesystem volume is a confusing failure — so they go too. Scoped to the
# release namespace: the substrate keeps its own in `mzmon-cloud`.
e2e-tier1-down:
	-helm uninstall mzmon $(HELM_KUBE) --namespace monitoring --wait --timeout 5m
	-helm uninstall mzmon-crds $(HELM_KUBE) --namespace monitoring --wait --timeout 5m
	$(KUBECTL) delete pvc -n monitoring --all --ignore-not-found
.PHONY: e2e-tier1-down

# Tier 2: the substrate, then the monitoring module composed onto it.
#
# Two applies rather than one root, because the substrate configures its own
# providers so it stays applyable alone; the tier-2 root reads its state. That is
# also why this depends on `e2e-generic-cloud` rather than duplicating it.
e2e-tier2: e2e-generic-cloud
	( cd terraform/test/tier2 && \
		$(TERRAFORM) init -input=false >/dev/null && \
		$(TERRAFORM) apply -auto-approve -input=false \
			-var 'kube_context=$(KIND_CONTEXT)' )
.PHONY: e2e-tier2

# Assert the tier-2 stack. Same binary and same target as tier 1 — the Thanos
# assertions that report `ignored` there run here, because the values now enable
# Thanos.
e2e-verify-tier2: E2E_DIAGNOSTICS_DIR = e2e-diagnostics-tier2
e2e-verify-tier2: e2e-verify
.PHONY: e2e-verify-tier2

e2e-tier2-down:
	( cd terraform/test/tier2 && \
		$(TERRAFORM) destroy -auto-approve -input=false \
			-var 'kube_context=$(KIND_CONTEXT)' )
.PHONY: e2e-tier2-down

### HUGO DOCS ###

serve-docs:
	$(HUGO_BIN) --source docs serve --gc --buildDrafts --openBrowser
.PHONY: serve-docs

docs/assets/dashboards/grafana: $(SOURCES_grafana-dashboards)
	mkdir -p "$@"
	rm -f "$@/"*.json
	$(PY_RUN) -m dashboards.render -o "$@" --format json
	$(PY_RUN) -m dashboards.render -o "$@" --format json --cloud-hint gcp --prefix gcp-
	touch "$@"

# Generate docs
docs/public: \
		$(shell find docs/content) \
		docs/content/reference/helm/materialize-monitoring-values.md
	$(HUGO_BIN) --source docs --destination public
	touch "$@"
