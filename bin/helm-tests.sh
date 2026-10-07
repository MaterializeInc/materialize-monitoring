#!/usr/bin/env bash
# Run the helm-unittest suites of each chart, one `helm unittest` process per
# suite file, several at a time.
#
# Every test renders the whole chart, subcharts included, so the suites are
# CPU-bound, and one `helm unittest` process works through them one after
# another on about a core and a half. The suites are independent of each other:
# each writes only its own snapshot file under tests/__snapshot__/, so they run
# side by side, with `--update-snapshot` too.
#
# The largest suite files start first. The slowest suites are the ones with the
# most tests, and a long suite started last would finish alone after the others.
#
# Most of a suite's CPU goes to Go's garbage collector rather than to rendering.
# Once every core is busy that is time the other suites wait for, so GOGC
# defaults to 400: about a third less CPU for about twice the heap, which peaks
# near 1.2GB for the largest suite.
#
# Each suite's output is held back and printed in full only if the suite fails,
# so parallel suites do not interleave their output. A line per suite reports
# progress as each one finishes.
#
# Environment:
#   HELM                the helm binary (default: helm)
#   HELM_TESTS_JOBS     suites to run at once (default: the number of CPUs)
#   HELM_UNITTEST_ARGS  extra flags for every `helm unittest` call, word-split
#                       as the Makefile passes them (e.g. --update-snapshot)
#   GOGC                Go garbage collector target (default: 400)
#
# Usage:
#   ./bin/helm-tests.sh CHART_DIR [CHART_DIR...]
PROG=$0
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail

if [ "$#" -eq 0 ]; then
    _error "usage: ${PROG} CHART_DIR [CHART_DIR...]"
    exit 1
fi

HELM=${HELM:-helm}
HELM_TESTS_JOBS=${HELM_TESTS_JOBS:-$(getconf _NPROCESSORS_ONLN)}
HELM_UNITTEST_ARGS=${HELM_UNITTEST_ARGS:-}
export GOGC=${GOGC:-400}

_require_progs "${HELM}"
if ! "${HELM}" unittest --help >/dev/null 2>&1; then
    _error "the helm-unittest plugin is not installed; run \`make helm-unittest-install\`"
    exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

# The suites helm-unittest collects by default: tests/*_test.yaml, not recursive.
suites=()
for chart in "$@"; do
    found=0
    for suite in "${chart%/}"/tests/*_test.yaml; do
        [ -f "${suite}" ] || continue
        suites+=("${suite}")
        found=1
    done
    if [ "${found}" -eq 0 ]; then
        _error "no tests/*_test.yaml suites in ${chart}"
        exit 1
    fi
done

# Where one suite's output is kept, named after its path.
function _log_for() {
    echo "${WORK_DIR}/$(echo "$1" | tr / _).log"
}

# Run one suite, keep its output, and report one line. A failed suite is
# appended to $WORK_DIR/failed: one short line per append, so the parallel
# workers do not tear each other's writes.
function _run_suite() {
    local suite=$1
    local chart=${suite%/tests/*}
    local log
    log=$(_log_for "${suite}")
    local start=${SECONDS}
    local result=PASS
    # shellcheck disable=SC2086 # HELM_UNITTEST_ARGS is a flag list, split as Make splits it
    if ! "${HELM}" unittest ${HELM_UNITTEST_ARGS} -f "tests/${suite##*/}" "${chart}" >"${log}" 2>&1; then
        result=FAIL
        echo "${suite}" >>"${WORK_DIR}/failed"
    fi
    printf '%s  %s (%ss)\n' "${result}" "${suite}" "$((SECONDS - start))"
}
export -f _log_for _run_suite
export HELM HELM_UNITTEST_ARGS WORK_DIR

_info "running ${#suites[@]} helm-unittest suites, ${HELM_TESTS_JOBS} at a time"
# shellcheck disable=SC2016 # $1 is for the bash that xargs starts
for suite in "${suites[@]}"; do
    printf '%d\t%s\n' "$(wc -c <"${suite}")" "${suite}"
done | sort -rn | cut -f2- | tr '\n' '\0' \
    | xargs -0 -n 1 -P "${HELM_TESTS_JOBS}" bash -c '_run_suite "$1"' _

tests=$(awk '/^Tests:/ { n += $(NF - 1) } END { print n + 0 }' "${WORK_DIR}"/*.log)
if [ -s "${WORK_DIR}/failed" ]; then
    while read -r suite; do
        _error "${suite}"
        cat "$(_log_for "${suite}")"
    done < <(sort "${WORK_DIR}/failed")
    _error "$(wc -l <"${WORK_DIR}/failed" | tr -d ' ') of ${#suites[@]} suites failed (${tests} tests, ${SECONDS}s)"
    exit 1
fi
_info "${#suites[@]} suites passed (${tests} tests, ${SECONDS}s)"
