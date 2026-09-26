#!/usr/bin/env bash
# Check the Alertmanager configuration this chart renders, with `amtool`.
#
# The chart generates Alertmanager's routing tree from `alerting` and passes
# receiver bodies through verbatim (templates/alertmanager-config.yaml). Helm
# cannot run `amtool` at render time, and the chart's own validators check only
# the structure they know about. A configuration Alertmanager rejects fails
# quietly at runtime: on a reload the previous configuration keeps running, and on
# a fresh start the pods crash-loop. So this renders the chart once per scenario,
# extracts `alertmanager.yml` and the notification templates from the
# `alertmanager-config` Secret, and runs `amtool check-config` over them.
#
# The default checker is the `amtool` inside the Alertmanager image the chart
# pins, run through docker, so the checker and the Alertmanager that will load the
# file agree on what is valid. The rendered files are mounted at
# /etc/alertmanager/config, the path the configuration names its templates under,
# so templates are loaded and checked too.
#
# Scenarios: the chart defaults, plus every file matching
# charts/materialize-monitoring/tests/alertmanager/*.values.yaml, each layered
# over the azure example profile (which enables the bundled backends).
#
# Environment:
#   AMTOOL        run this instead of docker; it is called as
#                 `$AMTOOL check-config <file>`. Templates are then not resolved.
#   AM_IMAGE      the image to take amtool from (default: alertmanager.image in values.yaml)
#   DOCKER, HELM  binaries to use
#
# Usage:
#   ./bin/check-alertmanager-config.sh
PROG=$0
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail

HELM=${HELM:-helm}
DOCKER=${DOCKER:-docker}
AMTOOL=${AMTOOL:-}
CHART_DIR=${CHART_DIR:-charts/materialize-monitoring}
SCENARIO_DIR="${CHART_DIR}/tests/alertmanager"
BASE_PROFILE="${CHART_DIR}/profiles/azure-example.values.yaml"

if [ -z "${AM_IMAGE:-}" ]; then
    # The chart pins the image in its own values.yaml, where Renovate bumps it,
    # so read it from there rather than restating a tag that would drift.
    AM_IMAGE=$(yq -r '.alertmanager.image.repository + ":" + .alertmanager.image.tag' "${CHART_DIR}/values.yaml")
    case "${AM_IMAGE}" in
        *null* | :* | *:)
            _error "could not read alertmanager.image.repository and .tag from ${CHART_DIR}/values.yaml (got ${AM_IMAGE})"
            exit 1
            ;;
    esac
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

if [ -n "${AMTOOL}" ]; then
    _require_progs "${HELM}" yq
else
    _require_progs "${HELM}" "${DOCKER}" yq
fi

# Render one scenario and extract the Secret's keys into $WORK_DIR/<name>/.
function _extract() {
    local name=$1
    shift
    local out="${WORK_DIR}/${name}"
    mkdir -p "${out}"
    if ! "${HELM}" template mzmon "${CHART_DIR}" \
        --namespace monitoring \
        -f "${BASE_PROFILE}" "$@" \
        >"${out}.render.yaml" 2>"${out}.err"; then
        _error "chart render failed for scenario '${name}'"
        cat "${out}.err" >&2
        return 1
    fi
    local secret='select(.kind == "Secret" and .metadata.name == "alertmanager-config")'
    local key
    while IFS= read -r key; do
        [ -n "${key}" ] || continue
        yq -r "${secret} | .stringData[\"${key}\"]" "${out}.render.yaml" >"${out}/${key}"
    done < <(yq -r "${secret} | .stringData | keys | .[]" "${out}.render.yaml")
    [ -s "${out}/alertmanager.yml" ] || {
        _error "scenario '${name}' rendered no alertmanager-config Secret"
        return 1
    }
}

function _check() {
    local name=$1
    local out="${WORK_DIR}/${name}"
    if [ -n "${AMTOOL}" ]; then
        ${AMTOOL} check-config "${out}/alertmanager.yml"
    else
        "${DOCKER}" run --rm \
            --volume "${out}:/etc/alertmanager/config:ro" \
            --entrypoint /bin/amtool \
            "${AM_IMAGE}" \
            check-config /etc/alertmanager/config/alertmanager.yml
    fi
}

_info "checking rendered Alertmanager configuration with amtool from ${AMTOOL:-${AM_IMAGE}}"
status=0
scenarios=("defaults")
_extract defaults || status=1
for f in "${SCENARIO_DIR}"/*.values.yaml; do
    [ -f "${f}" ] || continue
    name=$(basename "${f}" .values.yaml)
    scenarios+=("${name}")
    _extract "${name}" -f "${f}" || status=1
done
[ "${status}" -eq 0 ] || exit "${status}"

for name in "${scenarios[@]}"; do
    _info "==> ${name}"
    _check "${name}" || {
        _error "amtool rejected the configuration rendered for '${name}' (${PROG})"
        status=1
    }
done
exit "${status}"
