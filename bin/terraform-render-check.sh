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

    # A storageClass written to a path no subchart reads renders fine and is
    # silently ignored, so rendering alone proves nothing here. When an example
    # sets one, require every volumeClaimTemplate to carry it.
    #
    # The key may be quoted or not (Terraform's `yamlencode` quotes every key).
    # `|| true` because most examples set none, and a no-match grep would take
    # the script down under `pipefail`.
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

    # node-exporter's toggle writes the chart's circuit breaker rather than a
    # tag, because tags are OR'd and the chart carries it under `default`. That
    # makes the failure mode specific: `tags.node-exporter = false` is valid
    # YAML, reaches a key the chart really reads, and does nothing at all. The
    # DaemonSet either renders or it does not, so assert on that directly.
    # The composed documents are YAML, not JSON, so this reads them as text
    # rather than through jq. Terraform's `yamlencode` quotes every key, hence
    # the optional quotes. `tail -1` mirrors Helm's own last-document-wins merge,
    # so a caller overriding this through `additional_values` is respected.
    want_ne="$(grep -h -A2 '^"\?node-exporter"\?:' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null \
        | grep -oE '"?enabled"?:[[:space:]]*(true|false)' | grep -oE '(true|false)' | tail -1 || true)"
    if [ -z "${want_ne}" ]; then
        echo "  !! ${example}: the module wrote no top-level node-exporter.enabled" >&2
        echo "     install_node_exporter has to write the circuit breaker. A tag cannot" >&2
        echo "     switch it off, so moving it under tags: makes the input silently inert." >&2
        status=1
        continue
    fi
    got_ne="no"
    grep -q '^  name: node-exporter$' "${rendered}" && got_ne="yes"
    if [ "${want_ne}" = "true" ] && [ "${got_ne}" != "yes" ]; then
        echo "  !! ${example}: install_node_exporter is true but no node-exporter rendered" >&2
        status=1
        continue
    fi
    if [ "${want_ne}" != "true" ] && [ "${got_ne}" = "yes" ]; then
        echo "  !! ${example}: install_node_exporter is false but node-exporter still rendered" >&2
        echo "     A tag cannot switch it off; the circuit breaker is what does." >&2
        status=1
        continue
    fi
    echo "    node-exporter enabled=${want_ne}, rendered=${got_ne}"

    # Same reasoning for the Google Cloud Monitoring exporter: the observable
    # proof it landed is the per-destination filter env var, which only renders
    # when the chart actually sees the exporter enabled.
    if grep -q '"googleCloudExporter"' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null; then
        if ! grep -q 'GATEWAY_UNFILTERED_GCM_METRICS:' "${rendered}"; then
            echo "  !! ${example}: googleCloudExporter is set but no GCM metric filter rendered" >&2
            status=1
            continue
        fi
        echo "    GCM exporter reached the gateway pipeline"
    fi

    # Grafana's state database. Three things have to land together and each is
    # written to a different subchart path, so any one of them missing is a
    # Grafana that comes up on SQLite — or crash-loops on a half-written config —
    # while the plan looks entirely correct.
    if grep -q '"grafana_database_host"\|"host":.*:5432' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null \
        || grep -q '"database"' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null; then
        missing=""
        grep -q 'type = postgres' "${rendered}" || missing="${missing} grafana.ini[database]"
        # -F: the pattern is a literal. `$__file{...}` is Grafana's own expansion
        # syntax, read by Grafana at startup — not by the shell or by grep.
        # shellcheck disable=SC2016 # the literal $ is the point
        grep -qF 'password = $__file{/etc/secrets/grafana-db/password}' "${rendered}" || missing="${missing} password-file-ref"
        grep -q 'mountPath: /etc/secrets/grafana-db' "${rendered}" || missing="${missing} secret-mount"
        # The Secret itself is a Terraform resource, so it is never in `helm
        # template` output. What the chart can prove is that the mount names the
        # Secret Terraform creates — a mismatch there is a pod stuck on a volume
        # that does not exist.
        grep -q 'secretName: mzmon-grafana-db' "${rendered}" || missing="${missing} mount-names-tf-secret"

        if [ -n "${missing}" ]; then
            echo "  !! ${example}: grafana database configured but missing:${missing}" >&2
            echo "     The password never reaches grafana.ini as a literal by design, so the" >&2
            echo "     mount and the Secret are what make the [database] block usable." >&2
            status=1
            continue
        fi
        echo "    grafana database reached grafana.ini and the Secret mount"
    fi
done

if [ "${status}" -ne 0 ]; then
    echo "terraform render check FAILED" >&2
    exit "${status}"
fi

echo "terraform render check OK"
