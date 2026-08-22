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
# Matches the Makefile's PY_RUN. Used only by the scheduling assertion, which
# needs a YAML parser that jq cannot provide.
PY_RUN="${PY_RUN:-uv run}"

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

    # Scheduling has the same failure mode as the storage class — a selector
    # written to a path no subchart reads renders fine and does nothing — plus a
    # wrinkle that makes counting insufficient: two DaemonSets are *supposed* to
    # lack the node selector, because narrowing a per-node collector to a
    # workload pool is a silent blind spot rather than a placement choice. So
    # assert the exact set rather than a total, which catches a workload that
    # missed the fan-out *and* a DaemonSet that wrongly picked it up.
    #
    # Needs YAML, which jq cannot read; `uv run` matches the Makefile's PY_RUN.
    #
    # The exit code is inspected rather than just tested, so a missing interpreter
    # does not masquerade as a failed assertion: the checker returns 1 only when a
    # workload is genuinely misconfigured, and anything else (127 for a `uv` that
    # is not installed, or a crash) is a tooling problem that deserves to say so.
    scheduling_rc=0
    ${PY_RUN} python "${REPO_ROOT}/bin/check_scheduling_fanout.py" \
        "${rendered}" "${WORK_DIR}/${example}"-[0-9]*.yaml || scheduling_rc=$?

    if [ "${scheduling_rc}" -eq 1 ]; then
        echo "  !! ${example}: scheduling fan-out is incomplete." >&2
        echo "     See scheduling.tf and profiles/scheduling.values.yaml." >&2
        status=1
        continue
    elif [ "${scheduling_rc}" -ne 0 ]; then
        echo "  !! could not run the scheduling check: '${PY_RUN} python' exited ${scheduling_rc}." >&2
        echo "     This is a tooling problem, not a fan-out failure. Install uv" >&2
        echo "     (https://docs.astral.sh/uv/), or set PY_RUN to something that" >&2
        echo "     can run bin/check_scheduling_fanout.py with pyyaml available." >&2
        status=1
        continue
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

    # Loki's S3 endpoint, which has no default inside Loki and is not derivable
    # from the bucket or the region: the client rejects an empty one up front
    # ("create bucket: no s3 endpoint in config file") instead of falling through
    # to the AWS SDK, and every component that touches storage crash-loops. The
    # chart ships a default, so what this proves is that the module's own
    # `object_store` document did not overwrite it — a values-level regression
    # that renders perfectly and only fails in the cluster.
    #
    # Two clients are built off the same block by different helpers, so both are
    # checked; `backend:` gates the whole thing to the s3 examples.
    # The endpoint the module wrote is what must appear, not merely *an*
    # endpoint: the chart's default would satisfy a bare presence check even if
    # the module contributed nothing.
    # Static object-storage credentials (DEP-203).
    #
    # Gated on the *module call* declaring the variable, read from the plan's
    # configuration rather than from the rendered output. Gating on the output
    # would be circular: if the module stopped composing the credential, the gate
    # would stop firing and the check would pass. The first version of this did
    # exactly that and reported OK against a deliberately broken module.
    expected_key="$(jq -r '
        .configuration.root_module.module_calls.monitoring.expressions
        .object_storage_access_key_id.constant_value // empty
    ' "${plan_json}" 2>/dev/null || true)"

    if [ -n "${expected_key}" ]; then
        # Three things land, each failing differently: the key in Loki's own s3
        # block (chunk writes 403), the key in the Thanos objstore document
        # (receive and store fail the same way), and `configStorageType: Secret`.
        # That last one is not a correctness bug at all — everything works with it
        # wrong — it just publishes the secret key in a ConfigMap, which is
        # exactly why it needs an assertion rather than trust.
        # Asserted against the composed *values*, not the rendered manifests:
        # once the config lands in a Secret it is base64, so a plaintext grep of
        # the render would fail for the very reason the fix is correct.
        # `yamlencode` quotes every key, so these have to tolerate `"key": "value"`
        # as well as the bare form.
        missing_cred=""
        grep -hqE "\"?access_key_id\"?: \"?${expected_key}\"?" "${WORK_DIR}/${example}"-[0-9]*.yaml \
            || missing_cred="${missing_cred} loki-s3"
        grep -hqE "\"?access_key\"?: \"?${expected_key}\"?" "${WORK_DIR}/${example}"-[0-9]*.yaml \
            || missing_cred="${missing_cred} thanos-objstore"

        if [ -n "${missing_cred}" ]; then
            echo "  !! ${example}: static credentials did not reach:${missing_cred}" >&2
            echo "     Those backends fall back to the default credential chain and fail" >&2
            echo "     to authenticate at pod start, not at plan time." >&2
            status=1
            continue
        fi

        # The security half, stated as what must *not* happen: the key may appear
        # in a Secret (Helm writes those with `stringData`, so cleartext in the
        # manifest is normal and it is still a Secret at rest) and nowhere else.
        # The chart defaults Loki's configStorageType to ConfigMap and the
        # rendered Loki config carries secret_access_key verbatim, so a
        # regression puts it in a ConfigMap — which works perfectly and publishes
        # the key to anyone with get on ConfigMaps. That is exactly the kind of
        # thing that needs an assertion rather than trust.
        expected_secret="$(jq -r '
            .configuration.root_module.module_calls.monitoring.expressions
            .object_storage_secret_access_key.constant_value // empty
        ' "${plan_json}" 2>/dev/null || true)"

        if [ -n "${expected_secret}" ]; then
            leaked="$(python3 -c '
import sys
rendered, needle = sys.argv[1], sys.argv[2]
bad = []
for doc in open(rendered).read().split("\n---"):
    if needle not in doc:
        continue
    kind = next((l.split(":", 1)[1].strip() for l in doc.splitlines()
                 if l.startswith("kind:")), "?")
    if kind == "Secret":
        continue
    name = next((l.split(":", 1)[1].strip() for l in doc.splitlines()
                 if l.strip().startswith("name:")), "?")
    bad.append(kind + "/" + name)
print(" ".join(sorted(set(bad))))
' "${rendered}" "${expected_secret}")"

            if [ -n "${leaked}" ]; then
                echo "  !! ${example}: the secret access key reached non-Secret objects:${leaked}" >&2
                echo "     Check that Loki's configStorageType is Secret; the chart default is" >&2
                echo "     ConfigMap, which works but publishes the key to the namespace." >&2
                status=1
                continue
            fi
        fi

        # A scheme in the endpoint has to be stripped and turned into the
        # insecure flag, in *both* backends. Checked structurally rather than by
        # grep: the two write the same key name in different places, so a flat
        # search is satisfied by either one alone — the first version of this
        # passed with Loki's flag deliberately removed, because Thanos still had
        # its own.
        #
        # Both halves fail differently and neither error names the value that
        # caused it: a surviving scheme is "Endpoint url cannot have fully
        # qualified paths" at startup, and a missing insecure flag is a TLS
        # handshake against a plaintext port.
        given_ep="$(jq -r '
            .configuration.root_module.module_calls.monitoring.expressions
            .object_storage.constant_value.endpoint // empty
        ' "${plan_json}" 2>/dev/null || true)"

        case "${given_ep}" in
            http://*)
                if ! ${PY_RUN} python - "${given_ep}" "${WORK_DIR}/${example}"-[0-9]*.yaml <<'PYEOF'; then
import sys, yaml

given = sys.argv[1]
bare = given.split("://", 1)[1]
found = {}

for path in sys.argv[2:]:
    with open(path) as fh:
        doc = yaml.safe_load(fh) or {}
    s3 = (doc.get("loki", {}).get("loki", {}).get("storage", {})
             .get("object_store", {}).get("s3"))
    if isinstance(s3, dict):
        found["loki"] = s3
    cfg = doc.get("thanos", {}).get("global", {}).get("objstore", {}).get("config")
    if isinstance(cfg, str):
        found["thanos"] = (yaml.safe_load(cfg) or {}).get("config", {})

problems = []
for name in ("loki", "thanos"):
    block = found.get(name)
    if block is None:
        problems.append(f"{name}: no s3 config in the composed values")
        continue
    endpoint = str(block.get("endpoint", ""))
    if "://" in endpoint:
        problems.append(f"{name}: endpoint kept its scheme ({endpoint})")
    elif endpoint != bare:
        problems.append(f"{name}: endpoint is {endpoint!r}, expected {bare!r}")
    if block.get("insecure") is not True:
        problems.append(f"{name}: insecure is {block.get('insecure')!r}, expected True")

for p in problems:
    print(f"     {p}", file=sys.stderr)
sys.exit(1 if problems else 0)
PYEOF
                    echo "  !! ${example}: the http:// endpoint did not compose correctly" >&2
                    status=1
                    continue
                fi
                echo "    http:// endpoint reached both backends bare, with insecure set"
                ;;
        esac

        # Loki's egress policy has to name the port the endpoint is actually
        # on, *and* keep 443 for STS. Hardcoded at 443 it blocks any self-hosted
        # store, and the symptom names nothing useful: a bare `i/o timeout` in
        # the index gateway and every query hanging to a 504. Swapped to only the
        # store's port it would break workload identity instead, since the same
        # rule is what reaches STS.
        #
        # Read structurally, because a grep for a port number matches anything on
        # the page — including the endpoint it came from.
        if ! ${PY_RUN} python - "${given_ep}" "${WORK_DIR}/${example}"-[0-9]*.yaml <<'PYEOF'; then
import sys, yaml

given = sys.argv[1]
tail = given.rsplit(":", 1)[-1]
want = int(tail) if tail.isdigit() else (80 if given.startswith("http://") else 443)

ports = None
for path in sys.argv[2:]:
    with open(path) as fh:
        doc = yaml.safe_load(fh) or {}
    ext = (doc.get("loki", {}).get("networkPolicy", {}).get("externalStorage"))
    if isinstance(ext, dict) and ext.get("ports") is not None:
        ports = [int(p) for p in ext["ports"]]

if ports is None:
    sys.exit(0)  # no policy composed for this example; nothing to check

problems = []
if want not in ports:
    problems.append(f"object storage is on :{want} but egress allows {ports}")
if 443 not in ports:
    problems.append(f"443 missing from {ports}; STS is always 443, so workload identity breaks")

for p in problems:
    print(f"     {p}", file=sys.stderr)
sys.exit(1 if problems else 0)
PYEOF
            echo "  !! ${example}: Loki's external-storage egress is wrong" >&2
            status=1
            continue
        fi
        echo "    Loki's external-storage egress covers the endpoint and STS"

        echo "    static object-storage credentials reached loki, thanos, and a Secret"
    fi

    # --- certificates ---------------------------------------------------------
    # The class of bug tier 0 exists for: an issuer written to a path the chart
    # does not read is valid HCL, plans clean, and issues nothing. Only a render
    # shows whether the Certificate came out pointing at the right issuer.
    if grep -q '^kind: Certificate$' "${rendered}"; then
        cert_issuers="$(grep -A 3 '^[[:space:]]*issuerRef:$' "${rendered}" \
            | grep -oE 'name: "[^"]+"' | sed -E 's/name: "//; s/"$//' | sort -u || true)"

        if [ -z "${cert_issuers}" ]; then
            echo "  !! ${example}: Certificates rendered with no issuerRef" >&2
            status=1
            continue
        fi

        # Every component certificate must carry the full SAN ladder. A wrong or
        # short list installs clean and fails at the first handshake with a name
        # mismatch that reads like a broken certificate.
        if ! grep -q 'loki-distributor\.[a-z0-9-]*\.svc\.cluster\.local' "${rendered}"; then
            echo "  !! ${example}: Loki certificate is missing the fully-qualified SAN rung" >&2
            status=1
            continue
        fi
        if ! grep -q '^[[:space:]]*- loki-distributor$' "${rendered}"; then
            echo "  !! ${example}: Loki certificate is missing the bare-service SAN rung" >&2
            status=1
            continue
        fi

        echo "    certificates rendered, issuers: $(echo "${cert_issuers}" | tr '\n' ' ')"

        # Whichever issuer shape the example chose has to be the one that lands.
        if grep -q 'selfSigned' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null; then
            if ! grep -q 'kind: ClusterIssuer' "${rendered}"; then
                echo "  !! ${example}: asked for a self-signed root and no ClusterIssuer rendered" >&2
                status=1
                continue
            fi
            echo "    self-signed root bootstrapped as a ClusterIssuer"
        else
            expected_issuer="$(grep -hoE '"name": *"[^"]*"' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null \
                | sed -E 's/.*: *"//; s/"$//' | grep -E 'internal-ca|letsencrypt' | head -1 || true)"
            if [ -n "${expected_issuer}" ] && ! echo "${cert_issuers}" | grep -qx "${expected_issuer}"; then
                echo "  !! ${example}: issuer_ref ${expected_issuer} did not reach any Certificate" >&2
                status=1
                continue
            fi
            echo "    issuer_ref reached the component certificates"
        fi

        # The browser-facing certificate is the one that only exists behind an
        # L4 balancer, and it is separate because a public issuer cannot sign
        # in-cluster names.
        if grep -hq 'grafana_external_dns_names\|dnsNames' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null \
            && grep -hq '"external"' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null; then
            if ! grep -q 'grafana-external-tls' "${rendered}"; then
                echo "  !! ${example}: an external issuer and DNS names were set but no external certificate rendered" >&2
                status=1
                continue
            fi
            echo "    browser-facing certificate rendered for the external name"
        fi

        # `internal_tls` is the lever that composes the chart's mTLS profiles,
        # and every part of it is invisible to `terraform validate`: the profiles
        # are files this module reads at plan time, and a stage that failed to
        # compose renders a chart that installs perfectly and speaks plaintext.
        #
        # Read from the example's own HCL, not from the composed values: this is
        # a *module input*, and the values documents are its output. Grepping the
        # output for it would assert nothing — the whole question is whether the
        # input reached them.
        stage="$(grep -hoE '^[[:space:]]*internal_tls[[:space:]]*=[[:space:]]*"[^"]+"' \
            "${example_dir}"*.tf 2>/dev/null \
            | head -1 | sed -E 's/.*"([^"]+)"$/\1/' || true)"
        if [ -n "${stage}" ] && [ "${stage}" != "off" ]; then
            # Every marker below is matched as a *rendered setting*, never as a
            # bare substring. The chart emits validator warnings as YAML comments
            # that quote these same flag names back at the reader, so a loose
            # grep passes on the warning that says the flag is absent.
            #
            # Phase 1 is the floor for every non-`off` stage: Loki serves TLS on
            # its HTTP port and Thanos Receive on its remote-write listener.
            if ! grep -q '^[[:space:]]*cert_file: /etc/mzmon/tls/tls.crt$' "${rendered}"; then
                echo "  !! ${example}: internal_tls=${stage} but Loki's server TLS did not render" >&2
                status=1
                continue
            fi
            if ! grep -qE '^[[:space:]]*- "--remote-write.server-tls-cert=' "${rendered}"; then
                echo "  !! ${example}: internal_tls=${stage} but Thanos Receive's server TLS did not render" >&2
                status=1
                continue
            fi
            # The sizing profile also sets `thanos.receive.extraArgs`, and Helm
            # overwrites lists. Whichever document loses that merge does so
            # silently, and losing this line drops write quorum to 1.
            if ! grep -qE '^[[:space:]]*- "--receive.replication-factor=3"' "${rendered}"; then
                echo "  !! ${example}: internal_tls=${stage} clobbered the Thanos replication factor — the sizing profile and the mTLS profile are fighting over extraArgs" >&2
                status=1
                continue
            fi

            # Only `authenticate` refuses a client that presents nothing.
            # Asserting the negative for the earlier stages is the point of
            # having stages at all: `present` looks like mTLS in every values
            # file and rejects nothing, so a bug that skipped ahead to phase 3
            # would otherwise read as the feature working.
            alloy_requires='client_auth_type = "RequireAndVerifyClientCert"'
            thanos_requires='^[[:space:]]*- "--remote-write.server-tls-client-ca='
            if [ "${stage}" = "authenticate" ]; then
                if ! grep -qF "${alloy_requires}" "${rendered}"; then
                    echo "  !! ${example}: internal_tls=authenticate but no gateway listener requires a client certificate" >&2
                    status=1
                    continue
                fi
                if ! grep -qE "${thanos_requires}" "${rendered}"; then
                    echo "  !! ${example}: internal_tls=authenticate but Thanos Receive has no client CA, so it authenticates nobody" >&2
                    status=1
                    continue
                fi
            else
                if grep -qF "${alloy_requires}" "${rendered}" || grep -qE "${thanos_requires}" "${rendered}"; then
                    echo "  !! ${example}: internal_tls=${stage} should still serve a client presenting no certificate, but a listener requires one" >&2
                    status=1
                    continue
                fi
            fi

            # Loki's own hop tops out at verify-if-given, whatever the stage —
            # the kubelet probes that port and a httpGet probe cannot present a
            # certificate. `present` is where it arrives.
            if [ "${stage}" != "encrypt" ] \
                && ! grep -q '^[[:space:]]*client_auth_type: VerifyClientCertIfGiven$' "${rendered}"; then
                echo "  !! ${example}: internal_tls=${stage} but Loki's client CA policy did not render" >&2
                status=1
                continue
            fi
            echo "    internal_tls=${stage} composed the mTLS profiles onto the hops"
        fi
    fi

    if grep -q '^[[:space:]]*backend: s3$' "${rendered}"; then
        expected_ep="$(grep -hoE '"endpoint": *"[^"]+"' \
            "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null \
            | head -1 | sed -E 's/.*: *"//; s/"$//' || true)"

        if [ -z "${expected_ep}" ]; then
            echo "  !! ${example}: loki is on s3 but the module wrote no endpoint" >&2
            status=1
            continue
        fi

        missing_ep=""
        grep -A 4 '^[[:space:]]*object_store:$' "${rendered}" \
            | grep -q "endpoint: ${expected_ep}" || missing_ep="${missing_ep} chunk-client"
        grep -A 4 '^[[:space:]]*backend: s3$' "${rendered}" \
            | grep -q "endpoint: ${expected_ep}" || missing_ep="${missing_ep} ruler-storage"

        if [ -n "${missing_ep}" ]; then
            echo "  !! ${example}: loki is on s3 but ${expected_ep} did not reach:${missing_ep}" >&2
            echo "     Loki does not default one, so those components crash-loop with" >&2
            echo "     \"create bucket: no s3 endpoint in config file\"." >&2
            status=1
            continue
        fi
        echo "    loki s3 endpoint ${expected_ep} reached the chunk and ruler clients"
    fi

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

    if grep -q '"datadogExporter"' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null; then
        if ! grep -q 'GATEWAY_UNFILTERED_DATADOG_METRICS:' "${rendered}"; then
            echo "  !! ${example}: datadogExporter is set but no Datadog metric filter rendered" >&2
            status=1
            continue
        fi
        echo "    Datadog exporter reached the gateway pipeline"
    fi

    if grep -q '"otlpExporter"' "${WORK_DIR}/${example}"-[0-9]*.yaml 2>/dev/null; then
        if ! grep -q 'GATEWAY_UNFILTERED_OTLP_METRICS:' "${rendered}"; then
            echo "  !! ${example}: otlpExporter is set but no OTLP metric filter rendered" >&2
            status=1
            continue
        fi
        echo "    OTLP exporter reached the gateway pipeline"
    fi

    # Destination credentials (DEP-204).
    #
    # These never appear in the Helm values — they reach the gateway through a
    # Secret this module creates — so the coupling that can break is a *name*:
    # the values say which environment variable the pipeline reads, the Secret
    # says which one it sets, and for the OTLP header case the module derives
    # both from the header name. Nothing errors when those drift. The gateway
    # starts, `sys.env(...)` resolves empty, and the destination rejects every
    # request.
    #
    # So: every key of the Secret must be read by the rendered pipeline. That
    # catches a derived name changing on either side, and a Secret key nothing
    # consumes. Read from the plan rather than from the rendered output, which
    # never contains a Terraform-managed resource.
    cred_keys="$(jq -r '
        .planned_values.root_module.child_modules[].resources[]
        | select(.type == "kubernetes_secret" and .name == "alloy_gateway_env")
        | .values.data // {} | keys[]
    ' "${plan_json}" 2>/dev/null || true)"

    if [ -n "${cred_keys}" ]; then
        unread=""
        for key in ${cred_keys}; do
            grep -q "sys.env(\"${key}\")" "${rendered}" || unread="${unread} ${key}"
        done

        if [ -n "${unread}" ]; then
            echo "  !! ${example}: the gateway Secret sets variables the pipeline never reads:${unread}" >&2
            echo "     The values and the Secret have to agree on the name. They do not, so the" >&2
            echo "     destination authenticates with an empty credential at run time." >&2
            status=1
            continue
        fi

        # The security half, and the reason these are not passed as values at
        # all: `values` is recoverable with `helm get values` by anyone who can
        # read the release Secret. A credential that reached the rendered
        # manifests got there through the values, which is the regression.
        leaked_cred=""
        for key in ${cred_keys}; do
            value="$(jq -r --arg k "${key}" '
                .planned_values.root_module.child_modules[].resources[]
                | select(.type == "kubernetes_secret" and .name == "alloy_gateway_env")
                | .values.data[$k] // empty
            ' "${plan_json}" 2>/dev/null || true)"
            [ -n "${value}" ] || continue
            grep -qF -- "${value}" "${rendered}" && leaked_cred="${leaked_cred} ${key}"
        done

        if [ -n "${leaked_cred}" ]; then
            echo "  !! ${example}: destination credentials reached the Helm release:${leaked_cred}" >&2
            echo "     They belong in the module's Secret only — anything in values is readable" >&2
            echo "     with 'helm get values' and is stored in the release Secret besides." >&2
            status=1
            continue
        fi

        echo "    gateway credentials are read by the pipeline and absent from the release"
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
