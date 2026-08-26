#!/usr/bin/env bash
# Vendor the cog-generated Grafana schemas into this repo.
#
# grafana/cog is the codegen framework; it does not itself publish the Grafana
# resource schemas. The generated artifacts live in grafana/grafana-foundation-sdk,
# which runs cog over Grafana's cue/openapi sources and checks the results in.
# Those documents are the authoritative shape of what Grafana accepts, so we
# vendor them rather than re-running cog.
#
# Upstream publishes the same 55 documents twice: as OpenAPI 3.0 under `openapi/`
# and as JSON Schema draft-07 under `jsonschema/`. We vendor the draft-07 set:
# the type definitions are identical (verified name-for-name and property-for-
# property across all 55 pairs), draft-07 is what the Rust codegen consumes
# directly, and it expresses nullability as `anyOf [T, null]` rather than the
# OpenAPI 3.0 `nullable` keyword that draft-07 tooling ignores.
#
# The one thing only the OpenAPI set carries is each document's `info` block:
# x-schema-identifier (the Grafana plugin id), x-schema-kind, x-schema-variant.
# That is load-bearing -- the plugin id is what goes in a dashboard's
# VizConfigKind.group / DataQueryKind.group, and it is not always the document
# name (annotationslist -> `annolist`). So we fetch the OpenAPI set too, distill
# those three fields into packages.json, and keep only that.
#
# The upstream release tag is pinned below and maintained by Renovate (see the
# bin/*.sh customManager in renovate.json). Output is checked in, so a bump shows
# up as a reviewable schema diff; the auto-format workflow re-runs this script on
# PRs that touch the pin, since Renovate can only move the tag itself.
#
# Vendored bytes are kept byte-identical to upstream, which is why this directory
# is excluded from the formatters (see .pre-commit-config.yaml and .ecrc).
# Normalization that codegen needs happens in bin/gen-grafana-models.sh instead.

PROG=$0
# set cwd to repo root
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail
_register_traceback

FSDK_REPO="grafana/grafana-foundation-sdk"
# renovate: datasource=github-tags packageName=grafana/grafana-foundation-sdk
FSDK_REF="v0.0.18"

OUT_DIR="packages/mzmon-lib/schemas/grafana"
MANIFEST="$OUT_DIR/packages.json"

_require_progs curl jq

# api.github.com rate-limits unauthenticated calls per source IP, which is shared
# on CI runners. Use GITHUB_TOKEN when the environment offers one; the schemas are
# public, so an unset token only means the lower ceiling.
declare -a CURL_AUTH=()
if [ -n "${GITHUB_TOKEN:-}" ]; then
    CURL_AUTH=(--header "Authorization: Bearer $GITHUB_TOKEN")
    _info "Authenticating to api.github.com with GITHUB_TOKEN"
fi

_info "Vendoring Grafana schemas from $FSDK_REPO @ $FSDK_REF"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

# One tree call, filtered, beats a contents-API round trip per document.
_info "Listing jsonschema/ in the upstream tree"
curl -fsSL "${CURL_AUTH[@]}" \
    "https://api.github.com/repos/$FSDK_REPO/git/trees/$FSDK_REF?recursive=1" \
    -o "$WORK_DIR/tree.json"
# Sorted inside jq so the fetch order -- and any failure part-way through it --
# is reproducible.
mapfile -t names < <(jq -r '
    [ .tree[]
      | select(.type == "blob")
      | .path
      | select(startswith("jsonschema/") and endswith(".jsonschema.json"))
      | ltrimstr("jsonschema/") | rtrimstr(".jsonschema.json")
    ] | sort | .[]
' "$WORK_DIR/tree.json")

if [ "${#names[@]}" -eq 0 ]; then
    _error "no jsonschema/*.jsonschema.json blobs at $FSDK_REPO@$FSDK_REF"
    _error "upstream may have moved the schemas out of jsonschema/"
    exit 1
fi
_info "Found ${#names[@]} document(s)"

# Wipe first so packages dropped upstream do not linger as stale schemas. OUT_DIR
# holds nothing but this script's output, so taking the whole directory is safe --
# the hand-authored schemas/{alloy,query,scrape} are siblings, not children.
rm -rf "${OUT_DIR:?}"
mkdir -p "$OUT_DIR"

for name in "${names[@]}"; do
    curl -fsSL "https://raw.githubusercontent.com/$FSDK_REPO/$FSDK_REF/jsonschema/$name.jsonschema.json" \
        -o "$OUT_DIR/$name.jsonschema.json"
done
_info "Wrote ${#names[@]} document(s) to $OUT_DIR"

# Distill the plugin metadata the draft-07 renderings drop. Fetched into WORK_DIR
# and discarded -- only the three fields survive, in packages.json.
_info "Distilling plugin metadata from the OpenAPI renderings"
for name in "${names[@]}"; do
    curl -fsSL "https://raw.githubusercontent.com/$FSDK_REPO/$FSDK_REF/openapi/$name.openapi.json" \
        -o "$WORK_DIR/$name.openapi.json"
done

jq -n --arg ref "$FSDK_REF" '
    {
      _comment: (
        "GENERATED -- DO NOT EDIT. Plugin metadata distilled from the info block of "
        + "grafana-foundation-sdk openapi/*.openapi.json at \($ref). identifier is the "
        + "Grafana plugin id used as VizConfigKind.group (panelcfg) or DataQueryKind.group "
        + "(dataquery); it is not always the document name. An empty identifier means the "
        + "document carries no plugin identity (dashboardv2, common, units, ...)."
      ),
      packages: ($packages | add)
    }' \
    --slurpfile packages <(
        # One object per document; --slurpfile collects them into an array, and
        # `add` merges that array into a single map. No inner `jq -s` -- that
        # would nest an array inside the slurped array and `add` would just
        # concatenate it back to an array.
        for name in "${names[@]}"; do
            jq -c --arg n "$name" '{
                ($n): {
                    identifier: (.info["x-schema-identifier"] // ""),
                    kind: (.info["x-schema-kind"] // ""),
                    variant: (.info["x-schema-variant"] // "")
                }
            }' "$WORK_DIR/$name.openapi.json"
        done
    ) >"$MANIFEST"

_info "Wrote $MANIFEST ($(jq '.packages | length' "$MANIFEST") package(s))"

# Record where the tree came from; the schemas carry no version of their own
# (cog stamps no version into the draft-07 renderings at all).
curl -fsSL "${CURL_AUTH[@]}" \
    "https://api.github.com/repos/$FSDK_REPO/commits/$FSDK_REF" \
    -o "$WORK_DIR/commit.json"
read -r UPSTREAM_SHA UPSTREAM_DATE < <(
    jq -r '[.sha, .commit.committer.date] | @tsv' "$WORK_DIR/commit.json"
)

cat >"$OUT_DIR/PROVENANCE.md" <<EOF
# Provenance

GENERATED BY $PROG -- DO NOT EDIT.

Vendored copies of the cog-generated JSON Schema (draft-07) documents published
by grafana/grafana-foundation-sdk, plus \`packages.json\` -- plugin metadata
distilled from the \`info\` block of the parallel OpenAPI renderings, which the
draft-07 set drops.

| | |
| --- | --- |
| Upstream | https://github.com/$FSDK_REPO/tree/$FSDK_REF/jsonschema |
| Tag | \`$FSDK_REF\` |
| Commit | \`$UPSTREAM_SHA\` |
| Committed | $UPSTREAM_DATE |
| Documents | ${#names[@]} |

Re-vendor with \`$PROG\` after bumping \`FSDK_REF\`; Renovate maintains that pin.

The \`.jsonschema.json\` files are byte-identical to upstream. Codegen reads them
through the normalization in \`bin/gen-grafana-models.sh\` rather than patching
them here.
EOF
_info "Wrote $OUT_DIR/PROVENANCE.md"

_info "Done."
