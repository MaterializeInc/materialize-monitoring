#!/usr/bin/env bash
# Collect enough state to diagnose an E2E failure after the cluster is gone.
#
# A red E2E with no artifacts costs more than the test saves: the cluster is
# deleted with the runner, so anything not captured here is unrecoverable. Always
# exits 0 — a failure while collecting diagnostics must not mask the failure that
# triggered it.

set -uo pipefail

OUT_DIR="${1:-e2e-diagnostics}"
NAMESPACES="${NAMESPACES:-monitoring mzmon-cloud cnpg-system}"

mkdir -p "${OUT_DIR}"

kubectl get pods -A -o wide >"${OUT_DIR}/pods-all.txt" 2>&1
kubectl get events -A --sort-by=.lastTimestamp >"${OUT_DIR}/events-all.txt" 2>&1
kubectl get nodes -o wide >"${OUT_DIR}/nodes.txt" 2>&1
kubectl get sc,pv,pvc -A >"${OUT_DIR}/storage.txt" 2>&1

for ns in ${NAMESPACES}; do
    kubectl get all -n "${ns}" >"${OUT_DIR}/${ns}-resources.txt" 2>&1

    # Rendered config for the two ConfigMaps that never roll their pods — the
    # first thing to check when a config change appears not to have taken effect.
    for cm in mzmon-alloy-gateway mzmon-alloy-gateway-env mzmon-alloy-agent mzmon-alloy-agent-env loki; do
        kubectl get cm "${cm}" -n "${ns}" -o yaml >"${OUT_DIR}/${ns}-cm-${cm}.yaml" 2>/dev/null \
            || rm -f "${OUT_DIR}/${ns}-cm-${cm}.yaml"
    done

    # Describe only what is not healthy; a full describe of a working stack is
    # noise that makes the artifact harder to read.
    kubectl get pods -n "${ns}" --no-headers 2>/dev/null \
        | grep -vE 'Running|Completed' | awk '{print $1}' \
        | while read -r pod; do
            kubectl describe pod "${pod}" -n "${ns}" >"${OUT_DIR}/${ns}-describe-${pod}.txt" 2>&1
        done

    # Logs for every container, plus the previous instance when one crash-looped —
    # which is usually where the actual error is.
    kubectl get pods -n "${ns}" --no-headers -o custom-columns=:metadata.name 2>/dev/null \
        | while read -r pod; do
            [ -n "${pod}" ] || continue
            kubectl logs "${pod}" -n "${ns}" --all-containers --tail=2000 \
                >"${OUT_DIR}/${ns}-logs-${pod}.txt" 2>&1
            kubectl logs "${pod}" -n "${ns}" --all-containers --previous --tail=2000 \
                >"${OUT_DIR}/${ns}-logs-${pod}-previous.txt" 2>/dev/null \
                || rm -f "${OUT_DIR}/${ns}-logs-${pod}-previous.txt"
        done
done

helm list -A >"${OUT_DIR}/helm-releases.txt" 2>&1

echo "diagnostics written to ${OUT_DIR}"
exit 0
