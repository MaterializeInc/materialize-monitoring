#!/usr/bin/env bash
# Scan the rendered chart for Kubernetes misconfigurations with Trivy.
#
# Usage: bin/security-scan.sh [gate|report]
#
#   gate    (default) fail on HIGH/CRITICAL findings not baselined in
#           .trivyignore.yaml. This is what blocks a pull request.
#   report  write one SARIF file per scenario to $SARIF_DIR, unfiltered and at
#           every severity, for upload to GitHub code scanning.
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

status=0
[ "${MODE}" = "report" ] && mkdir -p "${SARIF_DIR}"

for scenario in "${SCENARIOS[@]}"; do
    name="${scenario%%:*}"
    profiles="${scenario#*:}"

    _info "==> ${name} (${profiles//,/ + })"
    _render "${name}" "${profiles}"

    case "${MODE}" in
        gate)
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
                _info "  no ${GATE_SEVERITY} findings outside the baseline"
            fi
            ;;
        report)
            # No --severity and no --ignorefile: the SARIF report is the
            # complete picture, and code scanning does its own triage.
            "${TRIVY}" config \
                --quiet \
                --format sarif \
                --output "${SARIF_DIR}/${name}.sarif" \
                "${WORK_DIR}/${name}.yaml"
            _info "  wrote ${SARIF_DIR}/${name}.sarif"
            ;;
        *)
            _error "unknown mode: ${MODE} (expected 'gate' or 'report')"
            exit 1
            ;;
    esac
done

if [ "${status}" -ne 0 ]; then
    _error "${PROG}: blocking misconfigurations found."
    _error "Fix them, or baseline an upstream finding in ${IGNORE_FILE} with a justification."
fi
exit "${status}"
