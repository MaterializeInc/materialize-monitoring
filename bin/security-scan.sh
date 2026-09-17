#!/usr/bin/env bash
# Scan the rendered chart with Trivy: Kubernetes misconfigurations, and
# vulnerabilities in the images the chart resolves to.
#
# Usage: bin/security-scan.sh [gate|report|images-gate|images-report]
#
#   gate           (default) fail on HIGH/CRITICAL misconfigurations not
#                  baselined in .trivyignore.yaml. Blocks a pull request.
#   report         write one SARIF file per scenario to $SARIF_DIR, unfiltered
#                  and at every severity, for upload to GitHub code scanning.
#   images-gate    fail on fixable HIGH/CRITICAL OS-package vulnerabilities in
#                  the images we build ourselves. Blocks a pull request.
#   images-report  write SARIF for every image the chart references, ours and
#                  upstream, at every severity, then merge them into one
#                  uploadable file ($MERGED_IMAGE_SARIF).
#
# Both gates are scoped to findings this repository can actually act on. The
# upstream population is large, moves on someone else's schedule, and is
# remediated by a Renovate bump rather than by a pull request -- gating on it
# would turn CI red on a dashboard change, which is how scanners get switched
# off. It reports instead.
#
# For the images we build, that scoping is narrower than it looks. `mzmon-alloy`
# is a Debian base plus the upstream Alloy binary, checksum-verified against
# Grafana's own SHA256SUMS. Vulnerabilities in the base layer we fix by
# rebuilding; vulnerabilities compiled into the Alloy binary (its Go stdlib and
# module graph) we cannot fix at all without Grafana cutting a release. So the
# image gate is `--pkg-types os --ignore-unfixed`: the base layer, and only
# where a fixed package version exists. The Go side reports.
#
# Images are scanned with `--image-src remote`, so what gets scanned is the
# published artifact rather than whatever a local Docker daemon happens to hold.
# That is also a correctness fix: a stale daemon copy of grafana/loki-canary
# failed to export its layers here, and Trivy prefers the daemon by default.
#
# We render with `helm template` and scan the result, rather than pointing
# `trivy config` at the chart directory. That is not a stylistic choice: Trivy's
# own Helm renderer cannot render this chart -- the Thanos subchart `fail`s
# without objstore config -- and it treats that as a WARNING, skips the chart,
# scans the static pre-rendered YAML instead, and exits 0. The result is a green
# check that has inspected none of the templates. Rendering here makes a render
# failure a hard error, which is the behavior a gate needs.
#
# Rendering also lets us scan more than the default values. Subchart enablement
# is tag-driven and several components are off by default, so a single default
# render leaves them uninspected. The scenarios below are chosen to cover
# different shapes rather than to enumerate every profile.

PROG=$0
# set cwd to repo root
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail

HELM=${HELM:-helm}
TRIVY=${TRIVY:-trivy}
CHART_DIR=${CHART_DIR:-charts/materialize-monitoring}
PROFILES_DIR="${CHART_DIR}/profiles"
IGNORE_FILE=${IGNORE_FILE:-.trivyignore.yaml}
SARIF_DIR=${SARIF_DIR:-trivy-sarif}
# Severities that block. Everything below this is reported, never gated: the
# LOW/MEDIUM population is dominated by upstream defaults we do not control.
GATE_SEVERITY=${GATE_SEVERITY:-HIGH,CRITICAL}
# Images matching this prefix are ours: we build and publish them, so we can fix
# a base-layer finding by rebuilding. Everything else is upstream.
OWN_IMAGE_PREFIX=${OWN_IMAGE_PREFIX:-ghcr.io/materializeinc/}
# Images that could not be scanned in images-report mode. Deliberately outside
# SARIF_DIR, which a CI job uploads wholesale, so nothing has to reason about
# which files in there are reports. A CI job checks this after the uploads.
FAILED_IMAGES_FILE=${FAILED_IMAGES_FILE:-trivy-failed-images.txt}
# images-report writes one SARIF per image, then merges them into this single
# file. Code scanning rejects an upload carrying several runs under one
# category, and a category per image would go stale as images come and go --
# see bin/merge_trivy_sarif.py.
MERGED_IMAGE_SARIF=${MERGED_IMAGE_SARIF:-trivy-images.sarif}

MODE=${1:-gate}

# Scenarios, as "name:profile[,profile...]". Profiles are applied in order, so
# the list reads the same way the -f flags do.
#
#   tier1   the local/dev shape, and what `make e2e-tier1` installs
#   azure   a cloud shape -- the widest render we have, with the bundled
#           backends and the operator path all on
#   mtls3   tier1 with mutual TLS at phase 3, where the listeners refuse a
#           client presenting no certificate
#
# Profiles that require operator-supplied values (aws-example and gcp-example
# both want Loki bucket names the Terraform module composes) are deliberately
# absent: the Terraform render check already covers that path.
SCENARIOS=(
    "tier1:loki-test,kind-tier1"
    "azure:azure-example"
    "mtls3:loki-test,kind-tier1,mtls-phase3"
)

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

# Render one scenario to $WORK_DIR/<name>.yaml.
function _render() {
    local name=$1 profiles=$2
    local helm_args=() profile
    while IFS= read -r profile; do
        helm_args+=(-f "${PROFILES_DIR}/${profile}.values.yaml")
    done < <(tr ',' '\n' <<<"${profiles}")

    if ! "${HELM}" template mzmon "${CHART_DIR}" \
        --namespace monitoring \
        "${helm_args[@]}" \
        >"${WORK_DIR}/${name}.yaml" 2>"${WORK_DIR}/${name}.err"; then
        _error "chart render failed for scenario '${name}'"
        cat "${WORK_DIR}/${name}.err" >&2
        return 1
    fi
    _info "  rendered $(grep -c '^kind:' "${WORK_DIR}/${name}.yaml") objects"
}

_require_progs "${HELM}" "${TRIVY}"
case "${MODE}" in
    images-gate | images-report) _require_progs yq ;;
esac

# The vendored subcharts are LFS-tracked. Checked out as pointer files they are
# small text blobs, and helm fails on them with an error that does not mention
# LFS at all, so check up front.
for tgz in "${CHART_DIR}"/charts/*.tgz; do
    if ! gzip -t "${tgz}" 2>/dev/null; then
        _error "subchart archive is not a gzip stream: ${tgz}"
        _error "these are LFS-tracked -- run 'git lfs pull' before scanning."
        exit 1
    fi
done

case "${MODE}" in
    gate | report | images-gate | images-report) ;;
    *)
        _error "unknown mode: ${MODE}"
        _error "expected one of: gate, report, images-gate, images-report"
        exit 1
        ;;
esac

status=0
case "${MODE}" in
    report | images-report) mkdir -p "${SARIF_DIR}" ;;
esac

# Render every scenario up front. The image modes need the whole set before they
# can build a deduplicated image list, and rendering is cheap next to scanning.
for scenario in "${SCENARIOS[@]}"; do
    name="${scenario%%:*}"
    profiles="${scenario#*:}"
    _info "==> render ${name} (${profiles//,/ + })"
    _render "${name}" "${profiles}"
done

# Every image any rendered object references, deduplicated.
#
# Read with yq rather than grepping for `image:`, so init containers, sidecars
# and any nesting a subchart invents are all caught. An image the chart pulls
# but no rendered object names -- a Job created at runtime, say -- is invisible
# here, which is a limitation of scanning the render rather than the cluster.
function _all_images() {
    yq ea '[.. | select(has("image")) | .image] | .[]' "${WORK_DIR}"/*.yaml \
        | grep -v '^null$' | sort -u
}

case "${MODE}" in
    gate)
        for scenario in "${SCENARIOS[@]}"; do
            name="${scenario%%:*}"
            # --exit-code 1 is what makes this a gate; without it Trivy prints
            # findings and exits 0.
            if ! "${TRIVY}" config \
                --quiet \
                --severity "${GATE_SEVERITY}" \
                --ignorefile "${IGNORE_FILE}" \
                --exit-code 1 \
                "${WORK_DIR}/${name}.yaml"; then
                _error "  ${name}: blocking findings above"
                status=1
            else
                _info "  ${name}: no ${GATE_SEVERITY} findings outside the baseline"
            fi
        done
        if [ "${status}" -ne 0 ]; then
            _error "${PROG}: blocking misconfigurations found."
            _error "Fix them, or baseline an upstream finding in ${IGNORE_FILE} with a justification."
        fi
        ;;

    report)
        # No --severity and no --ignorefile: the SARIF report is the complete
        # picture, and code scanning does its own triage.
        for scenario in "${SCENARIOS[@]}"; do
            name="${scenario%%:*}"
            "${TRIVY}" config \
                --quiet \
                --format sarif \
                --output "${SARIF_DIR}/${name}.sarif" \
                "${WORK_DIR}/${name}.yaml"
            _info "  wrote ${SARIF_DIR}/${name}.sarif"
        done
        ;;

    images-gate)
        own=0
        while IFS= read -r image; do
            case "${image}" in
                "${OWN_IMAGE_PREFIX}"*) ;;
                *) continue ;;
            esac
            own=$((own + 1))
            _info "==> ${image}"
            # --pkg-types os: the base layer, which a rebuild fixes.
            # --ignore-unfixed: no fixed version means no action to take, and a
            # gate that cannot be satisfied is one people route around.
            if ! "${TRIVY}" image \
                --quiet \
                --scanners vuln \
                --image-src remote \
                --pkg-types os \
                --ignore-unfixed \
                --severity "${GATE_SEVERITY}" \
                --exit-code 1 \
                "${image}"; then
                _error "  fixable ${GATE_SEVERITY} OS-package vulnerabilities above"
                status=1
            else
                _info "  no fixable ${GATE_SEVERITY} OS-package vulnerabilities"
            fi
        done < <(_all_images)

        if [ "${own}" -eq 0 ]; then
            # A rename of the published image would otherwise turn this gate
            # into a no-op that still reports success.
            _error "no images matched ${OWN_IMAGE_PREFIX} -- nothing was scanned."
            _error "If the published image moved, update OWN_IMAGE_PREFIX."
            exit 1
        fi
        if [ "${status}" -ne 0 ]; then
            _error "${PROG}: the base image is behind on a fixed package."
            _error "Rebuild and republish the image against a current base."
        fi
        ;;

    images-report)
        # A registry hiccup on one image should not cost us the other twelve
        # reports, so failures are collected rather than fatal. They are not
        # swallowed either: the list is written out, and CI fails on it after
        # the uploads have run.
        : >"${FAILED_IMAGES_FILE}"
        while IFS= read -r image; do
            # One SARIF file per image, named after it. Registry, path and tag
            # separators all become '-' so the name is a single path segment.
            slug="$(tr '/:@' '-' <<<"${image}")"
            _info "==> ${image}"
            if "${TRIVY}" image \
                --quiet \
                --scanners vuln \
                --image-src remote \
                --format sarif \
                --output "${SARIF_DIR}/image-${slug}.sarif" \
                "${image}"; then
                _info "  wrote ${SARIF_DIR}/image-${slug}.sarif"
            else
                _warning "could not scan ${image} -- continuing"
                echo "${image}" >>"${FAILED_IMAGES_FILE}"
                # A partial file from a failed run would upload as a clean
                # report, which is worse than no report at all.
                rm -f "${SARIF_DIR}/image-${slug}.sarif"
            fi
        done < <(_all_images)

        if [ -s "${FAILED_IMAGES_FILE}" ]; then
            _warning "$(wc -l <"${FAILED_IMAGES_FILE}" | tr -d ' ') image(s) could not be scanned; see ${FAILED_IMAGES_FILE}"
        fi

        # Stdlib-only, so this needs no Python environment set up in CI.
        python3 ./bin/merge_trivy_sarif.py "${SARIF_DIR}" "${MERGED_IMAGE_SARIF}"
        ;;
esac

exit "${status}"
