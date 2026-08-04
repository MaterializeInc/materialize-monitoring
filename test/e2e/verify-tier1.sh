#!/usr/bin/env bash
# Assert the tier-1 logging round trip: agent → gateway → Loki → query.
#
# A placeholder for `packages/mz-monitoring-e2e`, kept deliberately small. It
# exists so the CI job asserts something rather than only proving `helm install`
# exited zero — which it does even when no log ever reaches Loki, as the
# `loki-distributor` misconfiguration demonstrated.
#
# Every assertion retries against a deadline rather than polling once: ingestion
# is asynchronous, and a bare check here is the classic E2E flake.

set -euo pipefail

NAMESPACE="${NAMESPACE:-monitoring}"
TENANT="${TENANT:-loki}"
# Named explicitly rather than inherited: this port-forwards into a cluster, and
# defaulting to the current context would point it at whatever was last used.
# Empty means "current context", for the CI job where only one cluster exists.
KUBE_CONTEXT="${KUBE_CONTEXT:-}"
DEADLINE_SECONDS="${DEADLINE_SECONDS:-180}"
PORT="${PORT:-3100}"
# How recent a log line has to be to count as proof the write path is live. Wide
# enough to absorb the gateway's batch interval, narrow enough that chunks left
# from a previous run cannot satisfy it.
RECENT_WINDOW_SECONDS="${RECENT_WINDOW_SECONDS:-120}"

log() { echo "[verify-tier1] $*"; }
fail() {
    echo "[verify-tier1] FAIL: $*" >&2
    exit 1
}

# Retry `cmd` until it succeeds or the deadline passes.
retry_until() {
    local what="$1"
    shift
    local deadline=$((SECONDS + DEADLINE_SECONDS))
    local attempt=0
    until "$@"; do
        attempt=$((attempt + 1))
        if [ "${SECONDS}" -ge "${deadline}" ]; then
            fail "${what} did not succeed within ${DEADLINE_SECONDS}s (${attempt} attempts)"
        fi
        sleep 5
    done
    log "ok: ${what}"
}

KUBECTL=(kubectl)
[ -n "${KUBE_CONTEXT}" ] && KUBECTL+=(--context "${KUBE_CONTEXT}")

# Checked upfront, because otherwise a typo'd or missing context spends the whole
# deadline retrying a connection that can never succeed and reports it as a
# timeout — which reads like a broken stack rather than a broken invocation.
if [ -n "${KUBE_CONTEXT}" ] \
    && ! kubectl config get-contexts "${KUBE_CONTEXT}" >/dev/null 2>&1; then
    fail "kubeconfig has no context ${KUBE_CONTEXT} (create the cluster with 'make e2e-cluster')"
fi

# `config current-context` reports the *configured* context and ignores
# `--context`, so it would name the wrong cluster here. Echo what was requested.
log "cluster: ${KUBE_CONTEXT:-$(kubectl config current-context)}"

"${KUBECTL[@]}" -n "${NAMESPACE}" port-forward "svc/loki" "${PORT}:3100" >/dev/null 2>&1 &
PF_PID=$!
trap 'kill "${PF_PID}" 2>/dev/null || true' EXIT

api() { curl -sf -H "X-Scope-OrgID: ${TENANT}" "http://localhost:${PORT}$1"; }

check_ready() { curl -sf "http://localhost:${PORT}/ready" >/dev/null; }

# Non-zero streams is the assertion that the write path works. Loki answers
# label queries with `success` and no data when nothing has been ingested, so
# asserting on the query alone cannot distinguish "empty" from "broken".
check_streams() {
    local n
    n="$(curl -sf "http://localhost:${PORT}/metrics" 2>/dev/null \
        | awk '/^loki_ingester_streams_created_total/ {print $2; exit}')"
    [ -n "${n:-}" ] && [ "${n%.*}" -gt 0 ]
}

# The k8s_* families are applied by the gateway, not the agent, so their presence
# proves the relabelling stage ran rather than just that something was written.
check_labels() {
    api "/loki/api/v1/labels" | jq -e '.data | index("k8s_pod") != null' >/dev/null
}

# Bounded to a recent window, and this is the load-bearing assertion.
#
# An unbounded query is satisfied by chunks already on disk, so it passes against
# a stack whose write path is broken — verified by breaking it deliberately. Loki
# keeps its filesystem store across a pod restart and counts WAL-replayed streams
# in `streams_created_total`, so neither that counter nor an unbounded query can
# distinguish "ingesting now" from "ingested once, before the break".
check_recent_query() {
    local start
    start="$((($(date +%s) - RECENT_WINDOW_SECONDS) * 1000000000))"
    api "/loki/api/v1/query_range?query=%7Bnamespace%3D%22${NAMESPACE}%22%7D&limit=1&start=${start}" \
        | jq -e '.data.result | length > 0' >/dev/null
}

retry_until "loki /ready" check_ready
retry_until "loki ingested at least one stream" check_streams
retry_until "gateway-applied k8s_pod label is present" check_labels
retry_until "query_range returns a stream written in the last ${RECENT_WINDOW_SECONDS}s" check_recent_query

log "tier-1 round trip verified"
