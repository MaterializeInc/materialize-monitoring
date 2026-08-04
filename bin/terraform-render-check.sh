#!/usr/bin/env bash
# Plan each Terraform example, extract the Helm values the module composes, and
# render the chart against them.
#
# This is the tier-0 test from the Terraform module design doc, and it is the
# cheapest place to catch the whole class of bug the module can actually cause:
# a value written to a path the chart does not read, or a backend key left at a
# default that only happens to be right on one cloud. Rendering is what proves a
# value landed — `terraform validate` cannot, because every wrong path is still
# valid HCL, and `helm template` alone cannot, because it never sees what the
# module composed.
#
# No cluster required. The examples plan against a kubeconfig that does not
# exist: every resource is a create, nothing refreshes, so the providers are
# never asked to connect. That is deliberate — a check that needs a cluster does
# not run on a pull request.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHART_DIR="${REPO_ROOT}/charts/materialize-monitoring"
EXAMPLES_DIR="${REPO_ROOT}/terraform/modules/materialize-monitoring/examples"

TERRAFORM="${TERRAFORM:-terraform}"
HELM="${HELM:-helm}"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

VALUES_QUERY='[.planned_values.root_module.child_modules[].resources[]
               | select(.type == "helm_release" and .name == "monitoring")
               | .values.values[]]'

status=0

for example_dir in "${EXAMPLES_DIR}"/*/; do
    [ -f "${example_dir}/main.tf" ] || continue
    example="$(basename "${example_dir}")"

    echo "==> terraform plan ${example}"
    (
        cd "${example_dir}"
        "${TERRAFORM}" init -backend=false -input=false >/dev/null
        # A kubeconfig path that cannot exist, so a stray provider connection
        # fails loudly here rather than silently reaching the operator's own
        # cluster.
        "${TERRAFORM}" plan -input=false \
            -var "kubeconfig_path=${WORK_DIR}/no-such-kubeconfig" \
            -out "${WORK_DIR}/${example}.tfplan" >/dev/null
    )

    plan_json="${WORK_DIR}/${example}.json"
    "${TERRAFORM}" -chdir="${example_dir}" show -json "${WORK_DIR}/${example}.tfplan" >"${plan_json}"

    doc_count="$(jq -r "${VALUES_QUERY} | length" "${plan_json}")"
    if [ "${doc_count}" -eq 0 ]; then
        echo "  !! no helm_release values found in the plan for ${example}" >&2
        status=1
        continue
    fi

    # The module composes an ordered list of YAML documents, and Helm merges them
    # with later documents winning. Repeated `-f` has the same semantics, so
    # writing each out in order and passing them all reproduces the release.
    helm_args=()
    for i in $(seq 0 $((doc_count - 1))); do
        doc_file="${WORK_DIR}/${example}-${i}.yaml"
        jq -r --argjson i "${i}" "${VALUES_QUERY}[\$i]" "${plan_json}" >"${doc_file}"
        helm_args+=(-f "${doc_file}")
    done

    echo "==> helm template ${example} (${doc_count} value documents)"
    # The chart's own validators run at render time, which is the point: a
    # backend mismatch the module introduces fails here with the same message an
    # operator would get from `helm install`.
    if ! "${HELM}" template mzmon "${CHART_DIR}" \
        --namespace monitoring \
        "${helm_args[@]}" \
        >"${WORK_DIR}/${example}-rendered.yaml"; then
        echo "  !! chart render failed for ${example}" >&2
        status=1
        continue
    fi

    rendered="${WORK_DIR}/${example}-rendered.yaml"
    echo "    rendered $(grep -c '^kind:' "${rendered}") objects"

    # Rendering proves the chart accepts the values; it does not prove a value
    # reached the setting it was aimed at. A storageClass written to a path no
    # subchart reads renders perfectly and is silently ignored, and the PVC lands
    # on the cluster default instead — which on a Hyperdisk-only node pool means
    # it never attaches.
    #
    # So when an example sets one, require every volumeClaimTemplate in the
    # output to carry it. The count is the assertion: a PVC-backed workload the
    # fan-out map does not know about fails here rather than in a cluster.
    # The key may be quoted or not: Terraform's `yamlencode` quotes every key,
    # while a hand-written `additional_values` document usually does not.
    # `|| true` because most examples set no storage class, and a no-match grep
    # would take the whole script down under `pipefail`.
    expected_sc="$(grep -hoE '"?storageClass"?:[[:space:]]*"?[^"[:space:]]+' \
        "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null \
        | head -1 | sed -E 's/.*:[[:space:]]*"?//' || true)"

    if [ -n "${expected_sc}" ]; then
        want="$(grep -c 'volumeClaimTemplates:' "${rendered}" || true)"
        got="$(grep -c "storageClassName: ${expected_sc}" "${rendered}" || true)"
        if [ "${want}" != "${got}" ]; then
            echo "  !! ${example}: ${want} volumeClaimTemplates but ${got} carry storageClassName ${expected_sc}" >&2
            echo "     A PVC-backed workload is missing from storage_class.tf's fan-out." >&2
            status=1
            continue
        fi
        echo "    storageClass reached all ${got} volumeClaimTemplates"
    fi
done

if [ "${status}" -ne 0 ]; then
    echo "terraform render check FAILED" >&2
    exit "${status}"
fi

echo "terraform render check OK"
