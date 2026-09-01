#!/usr/bin/env bash
# Generate Rust models from the vendored Grafana JSON Schema documents.
#
# Each upstream document is self-contained -- there are no cross-document $refs,
# so shared types are duplicated into every document that uses them. Across the
# full set that is 1285 definitions under only 769 distinct names, and 44 of the
# duplicated names genuinely disagree (`Options` exists in 24 documents with 24
# different shapes; `FieldConfig` in 15 with 11). A flat namespace is therefore
# impossible: one Rust module per document is the only mapping that works, and it
# is what the upstream Go/Python SDKs do too.
#
# The same self-containment means `common` is not worth generating -- nothing can
# $ref it, and every panel document already carries its own copy of those types.
#
# PACKAGES is deliberately narrow. Generating all 53 usable documents costs ~92k
# lines of mostly redundant copies, so this list tracks what the dashboards
# actually emit.
# Add a document here to pull it in; the vendored tree always holds all 55.

PROG=$0
# set cwd to repo root
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail
_register_traceback

SCHEMA_DIR="packages/mzmon-lib/schemas/grafana"
# `gen` is a reserved keyword in edition 2024, so the module is `generated`.
OUT_DIR="packages/mzmon-lib/src/grafana/generated"

# Core dashboard kinds, the six panel plugins env-top.yaml renders, the log panel
# and Loki dataquery for log dashboards, and the Prometheus dataquery.
PACKAGES=(
    dashboardv2
    timeseries stat piechart table gauge barchart
    logs
    # `text` carries no data. It is how a row explains itself -- notably the
    # stand-in shown when a conditionally-rendered row is hidden.
    text
    prometheus loki
)

_require_progs jq cargo-typify

_info "Generating Rust models for ${#PACKAGES[@]} package(s)"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

# Wipe so a package dropped from PACKAGES does not linger as a stale module.
rm -rf "${OUT_DIR:?}"
mkdir -p "$OUT_DIR"

for pkg in "${PACKAGES[@]}"; do
    src="$SCHEMA_DIR/$pkg.jsonschema.json"
    if [ ! -f "$src" ]; then
        _error "no vendored schema for '$pkg' at $src"
        _error "run bin/fetch-grafana-schemas.sh, or fix the PACKAGES list"
        exit 1
    fi

    # Strip `default` keywords that sit beside a `$ref`. Draft-07 ignores $ref
    # siblings, so those defaults are inert by spec -- and cog emits some that
    # contradict the schema they point at (table's Options.footer defaults
    # `reducer` to null, where TableFooterOptions requires it to be an array).
    # typify validates defaults and rejects the document over it. Normalizing
    # here keeps the vendored bytes byte-identical to upstream.
    jq 'def clean:
          if type == "object" then
            (if has("$ref") and has("default") then del(.default) else . end)
            | with_entries(.value |= clean)
          elif type == "array" then map(clean)
          else . end;
        clean' "$src" >"$WORK_DIR/$pkg.json"

    # Widen spots where the upstream schema is narrower than Grafana actually is.
    # Unlike the $ref-sibling strip above, these are semantic corrections, so each
    # one is listed explicitly with the evidence that motivated it.
    #
    # MatcherConfig.options: typed `object`, but Grafana's `byName` matcher takes
    # the field name as a bare string -- `{"id": "byName", "options": "Errors"}`.
    # The pre-rendered env-top.yaml emits exactly that and Grafana accepts it, so
    # the schema is wrong and the generated model would be stricter than the
    # server. Widening to `any` yields serde_json::Value.
    #
    # DynamicConfigValue.value: typed `object`, but a field override's value is
    # whatever the property takes -- `{"id": "unit", "value": "bytes"}` is a bare
    # string, `custom.width` a number. Typed as an object the model can only
    # express the map case, so a scalar has to be wrapped, and Grafana renders the
    # wrapper as `[object Object]` in the cell. Widening to `any` yields
    # serde_json::Value and lets the scalar through.
    if [ "$pkg" = "dashboardv2" ] || [ "$pkg" = "dashboardv2beta1" ] || [ "$pkg" = "dashboard" ]; then
        jq '(.definitions.MatcherConfig.properties.options) |= {
                "description": (.description // "The matcher options. This is specific to the matcher implementation.")
            }
            | (.definitions.DynamicConfigValue.properties.value) |= {
                "description": (.description // "The property value. Shape depends on the property id.")
            }' "$WORK_DIR/$pkg.json" >"$WORK_DIR/$pkg.patched.json"
        mv "$WORK_DIR/$pkg.patched.json" "$WORK_DIR/$pkg.json"
    fi

    # BTreeMap, not the default HashMap: a dashboard's `elements` map is
    # serialized into a checked-in artifact, and HashMap's iteration order is
    # randomly seeded per process, so two identical builds would reorder the keys
    # and churn the diff.
    if ! cargo typify --no-builder \
        --additional-derive PartialEq \
        --map-type '::std::collections::BTreeMap' \
        --output "$OUT_DIR/$pkg.rs" "$WORK_DIR/$pkg.json" >"$WORK_DIR/$pkg.log" 2>&1; then
        _error "typify failed for '$pkg':"
        sed 's/\x1b\[[0-9;]*m//g' "$WORK_DIR/$pkg.log" | sed 's/^/    /' >&2
        exit 1
    fi
    # Banner, and a workaround in one. typify's first line is an inner attribute
    # (`#![allow(clippy::redundant_closure_call)]`), and pre-commit's
    # check-shebang-scripts-are-executable reads any file starting with `#!` as a
    # script missing its executable bit. A leading comment is legal before an
    # inner attribute, so the banner these files should carry anyway also moves
    # the `#!` off line 1. Do not "fix" this by exempting the hook -- it guards
    # the executable bit on every real script under bin/ and tools/.
    {
        echo "// GENERATED BY $PROG -- DO NOT EDIT."
        echo "// Source: $src"
        cat "$OUT_DIR/$pkg.rs"
    } >"$WORK_DIR/$pkg.rs.banner"
    mv "$WORK_DIR/$pkg.rs.banner" "$OUT_DIR/$pkg.rs"

    _info "  $pkg -> $OUT_DIR/$pkg.rs ($(wc -l <"$OUT_DIR/$pkg.rs" | tr -d ' ') lines)"
done

# Module index, with each package's Grafana plugin id from the vendored manifest.
# That id is what a dashboard puts in VizConfigKind.group / DataQueryKind.group,
# and it is not always the document name (annotationslist -> `annolist`), so it is
# worth surfacing next to the module rather than leaving callers to guess.
{
    echo "//! Generated Grafana models -- DO NOT EDIT."
    echo "//!"
    echo "//! Regenerate with \`$PROG\`. One module per upstream schema document;"
    echo "//! see that script for why a flat namespace is not possible."
    echo
    # This crate is linted with `clippy -D warnings` on pre-push, and typify's
    # output trips three lints by construction: it emits hand-written Default
    # impls where a derive would do, unboxed enum variants of uneven size, and
    # per-document default helpers that only some documents call. None are
    # actionable in generated code, and the alternative -- teaching the generator
    # or post-processing its output -- buys nothing.
    echo "#![allow(dead_code, clippy::derivable_impls, clippy::large_enum_variant)]"
    echo
    # Separator goes before each entry rather than after, so the file does not end
    # on a blank line -- end-of-file-fixer trims that, and this directory is not
    # excluded from the formatters (unlike the vendored schemas), so a trailing
    # blank would be re-added by every run and stripped by every commit.
    first=true
    for pkg in "${PACKAGES[@]}"; do
        [ "$first" = true ] || echo
        first=false
        id=$(jq -r --arg p "$pkg" '.packages[$p].identifier' "$SCHEMA_DIR/packages.json")
        variant=$(jq -r --arg p "$pkg" '.packages[$p].variant' "$SCHEMA_DIR/packages.json")
        if [ -n "$id" ]; then
            echo "/// Grafana plugin id \`$id\` (${variant:-core})."
            echo "pub const ${pkg^^}_PLUGIN_ID: &str = \"$id\";"
        fi
        echo "pub mod $pkg;"
    done
} >"$OUT_DIR/mod.rs"
_info "Wrote $OUT_DIR/mod.rs"

_info "Done. Run \`cargo check -p mzmon-lib\` to verify."
