#!/usr/bin/env bash
# Check the alerting rules this chart renders: PromQL with `promtool`, LogQL with
# Loki's own parser.
#
# The rules are generated from the query registry into
# charts/materialize-monitoring/pre-rendered/rules/{prometheus,loki}/, where their
# deployment-specific values are still placeholders, and the chart fills those in
# and filters the set (templates/alerts/prometheusrules.yaml and lokirules.yaml).
# A rule the ruler rejects is dropped along with its whole group, and nothing
# reports it; a rule that parses but can never fire looks exactly like one with
# nothing to report. So this renders the chart once per scenario, extracts each
# PrometheusRule's groups, the PromQL and LogQL ones apart by their
# `mzmon.materialize.cloud/flavor` label, and runs:
#
#   * `promtool check rules` over every scenario's PromQL rules, so the filled-in
#     rules parse under each shape of values;
#   * `logcli query --stdin` over every scenario's LogQL rule expressions, which
#     parses each with Loki's LogQL parser and needs no Loki; and
#   * `promtool test rules` over packages/queries/tests/*.test.yaml, against the
#     `all` scenario (every rule, chart-default namespaces), so each PromQL rule's
#     behaviour is pinned by an input that should fire it and one that should not.
#     Loki has no equivalent for LogQL rules.
#
# Scenarios: every file matching charts/materialize-monitoring/tests/rules/*.values.yaml,
# each layered over the azure example profile (which enables the bundled
# backends). `all.values.yaml` selects every rule and is the one the unit tests
# run against; the others each layer over it.
#
# The default checkers are `promtool` from the Prometheus image and `logcli` from
# the Loki release pinned below, run through docker, so a missing or mismatched
# local install cannot change the result. promtool matches the one CI installs
# for the cargo tests.
#
# Environment:
#   PROMTOOL      run this instead of docker (e.g. `promtool`)
#   LOGCLI        run this instead of docker (e.g. `logcli`)
#   PROM_IMAGE    the image to take promtool from
#   LOGCLI_IMAGE  the image to take logcli from
#   DOCKER, HELM  binaries to use
#
# Usage:
#   ./bin/check-rules.sh
PROG=$0
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail

HELM=${HELM:-helm}
DOCKER=${DOCKER:-docker}
PROMTOOL=${PROMTOOL:-}
LOGCLI=${LOGCLI:-}
# Keep in step with the promtool the cargo-test job installs (.github/workflows/test.yaml).
# renovate: datasource=docker packageName=quay.io/prometheus/prometheus
PROM_VERSION=v3.12.0
PROM_IMAGE=${PROM_IMAGE:-quay.io/prometheus/prometheus:${PROM_VERSION}}
# Keep in step with the Loki the chart runs (`loki.loki.image.tag` in the
# chart's values.yaml). Renovate groups the two.
# renovate: datasource=docker packageName=grafana/logcli
LOGCLI_VERSION=3.7.8
LOGCLI_IMAGE=${LOGCLI_IMAGE:-docker.io/grafana/logcli:${LOGCLI_VERSION}}
CHART_DIR=${CHART_DIR:-charts/materialize-monitoring}
SCENARIO_DIR="${CHART_DIR}/tests/rules"
BASE_PROFILE="${CHART_DIR}/profiles/azure-example.values.yaml"
TEST_DIR=${TEST_DIR:-packages/queries/tests}

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

if [ -n "${PROMTOOL}" ] && [ -n "${LOGCLI}" ]; then
    _require_progs "${HELM}" yq
else
    _require_progs "${HELM}" "${DOCKER}" yq
fi

# Render one scenario and write each PromQL PrometheusRule's spec to
# $WORK_DIR/<name>/<registry-file>.yaml, named after the registry file it came
# from so the unit tests can name it in `rule_files`, and each LogQL one's to
# $WORK_DIR/<name>.loki/<registry-file>.yaml.
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
    mkdir -p "${out}.loki"
    local flavor='.metadata.labels["mzmon.materialize.cloud/flavor"]'
    local kind dir rules resource
    for kind in promql logql; do
        if [ "${kind}" = "logql" ]; then
            dir="${out}.loki"
            rules="select(.kind == \"PrometheusRule\" and ${flavor} == \"logql\")"
        else
            dir="${out}"
            rules="select(.kind == \"PrometheusRule\" and ${flavor} != \"logql\")"
        fi
        while IFS= read -r resource; do
            # yq separates documents with `---`, which is not a name.
            if [ -z "${resource}" ] || [ "${resource}" = "---" ]; then
                continue
            fi
            # The resource is `<fullname>-<registry file stem>`.
            yq -r "${rules} | select(.metadata.name == \"${resource}\") | .spec" "${out}.render.yaml" \
                >"${dir}/${resource#mzmon-}.yaml"
        done < <(yq -r "${rules} | .metadata.name" "${out}.render.yaml")
    done
    if [ -z "$(ls -A "${out}")" ]; then
        _error "scenario '${name}' rendered no PromQL PrometheusRule"
        return 1
    fi
}

function _promtool() {
    local dir=$1
    shift
    if [ -n "${PROMTOOL}" ]; then
        (cd "${dir}" && ${PROMTOOL} "$@")
    else
        "${DOCKER}" run --rm \
            --volume "${dir}:/work:ro" \
            --workdir /work \
            --entrypoint /bin/promtool \
            "${PROM_IMAGE}" \
            "$@"
    fi
}

# Parse one LogQL expression with Loki's parser. `logcli --stdin` evaluates a
# query against lines on stdin with no Loki, and parses it first. A metric query
# that parses is then refused as unsupported over stdin, and that refusal is what
# tells it apart from a log query, which runs, and which a ruler refuses.
function _logcli_parse() {
    local expr=$1
    local out
    if [ -n "${LOGCLI}" ]; then
        out=$(${LOGCLI} query --stdin --quiet "${expr}" </dev/null 2>&1 || true)
    else
        out=$("${DOCKER}" run --rm -i "${LOGCLI_IMAGE}" query --stdin --quiet "${expr}" </dev/null 2>&1 || true)
    fi
    if printf '%s' "${out}" | grep -q "parse error"; then
        printf '%s\n' "${out}" >&2
        return 1
    fi
    if ! printf '%s' "${out}" | grep -q "Query: not supported"; then
        printf 'parsed as a log query, not a metric query: %s\n' "${out}" >&2
        return 1
    fi
}

_info "checking rendered alerting rules with promtool from ${PROMTOOL:-${PROM_IMAGE}}"
status=0
scenarios=()
for f in "${SCENARIO_DIR}"/*.values.yaml; do
    [ -f "${f}" ] || continue
    name=$(basename "${f}" .values.yaml)
    scenarios+=("${name}")
    if [ "${name}" = "all" ]; then
        _extract "${name}" -f "${f}" || status=1
    else
        _extract "${name}" -f "${SCENARIO_DIR}/all.values.yaml" -f "${f}" || status=1
    fi
done
[ "${status}" -eq 0 ] || exit "${status}"

for name in "${scenarios[@]}"; do
    _info "==> check rules: ${name}"
    files=()
    for f in "${WORK_DIR}/${name}"/*.yaml; do
        files+=("$(basename "${f}")")
    done
    _promtool "${WORK_DIR}/${name}" check rules "${files[@]}" || {
        _error "promtool rejected the rules rendered for '${name}' (${PROG})"
        status=1
    }
done

_info "checking rendered LogQL rules with logcli from ${LOGCLI:-${LOGCLI_IMAGE}}"
if [ -z "$(ls -A "${WORK_DIR}/all.loki")" ]; then
    _error "scenario 'all' rendered no LogQL PrometheusRule"
    status=1
fi
for name in "${scenarios[@]}"; do
    _info "==> parse LogQL rules: ${name}"
    for f in "${WORK_DIR}/${name}.loki"/*.yaml; do
        [ -f "${f}" ] || continue
        count=$(yq '[.groups[].rules[]] | length' "${f}")
        for i in $(seq 0 $((count - 1))); do
            alert=$(yq -r "[.groups[].rules[]][${i}].alert" "${f}")
            expr=$(yq -r "[.groups[].rules[]][${i}].expr" "${f}")
            _logcli_parse "${expr}" || {
                _error "${alert} as rendered for '${name}' is not a rule the Loki ruler would load (${PROG})"
                status=1
            }
        done
    done
done

_info "==> test rules: ${TEST_DIR}"
tests=()
for t in "${TEST_DIR}"/*.test.yaml; do
    [ -f "${t}" ] || continue
    cp "${t}" "${WORK_DIR}/all/"
    tests+=("$(basename "${t}")")
done
if [ "${#tests[@]}" -gt 0 ]; then
    _promtool "${WORK_DIR}/all" test rules "${tests[@]}" || {
        _error "a rule unit test failed (${PROG})"
        status=1
    }
fi
exit "${status}"
