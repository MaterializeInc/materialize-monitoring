#!/usr/bin/env bash
# Populate charts/*/charts/ from Chart.lock without rewriting the lock.
#
# Usage: bin/helm-deps.sh CHART_DIR [CHART_DIR...]
#
# The list of charts lives in the Makefile (HELM_DEP_CHARTS) and is passed in,
# so there is one source of truth for which charts vendor their subcharts.

PROG=$0
# set cwd to repo root
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail

HELM=${HELM:-helm}
# Throwaway repository config, see _register_repos.
REPO_CONFIG=

# Print the classic (plain HTTP) chart repositories a chart's dependencies name,
# deduplicated. `oci://`, `file://`, and inline ("") repositories are excluded:
# helm resolves those without a repository entry.
function _classic_repos() {
    yq e '[.dependencies[] | .repository // "" | select(test("^https?://"))] | unique | .[]' "$1/Chart.yaml"
}

# Register every classic repository a chart depends on.
#
# `helm dependency build` refuses to run unless each one is already present in
# the repository config -- it fails with "no repository definition for ..."
# before it ever looks at the lock. A CI runner starts with an empty config, so
# without this the build step could never succeed there.
#
# The entries go in a throwaway config file so a developer's own `helm repo`
# names are neither read nor modified and CI and local runs resolve identically.
# The index cache is left at its default location so repeat runs reuse it.
#
# Names are derived from the URL rather than counted off, so two charts sharing a
# repository reuse one entry and two charts with different repositories never
# collide on a name. `--force-update` makes re-adding the same entry a no-op.
function _register_repos() {
    local chart=$1
    local url
    local name
    while read -r url; do
        [ -n "$url" ] || continue
        name=$(_repo_name "$url")
        _info "==> helm repo add $name $url"
        HELM_REPOSITORY_CONFIG="$REPO_CONFIG" "$HELM" repo add --force-update "$name" "$url" >/dev/null
    done < <(_classic_repos "$chart")
}

# Slugify a repository URL into a helm repo entry name.
function _repo_name() {
    printf '%s' "$1" | sed -e 's|^https\{0,1\}://||' -e 's|[^A-Za-z0-9]\{1,\}|-|g' -e 's|^-||' -e 's|-$||'
}

# Install the locked subchart versions into $chart/charts/.
#
# `build` is what we want: it installs exactly what Chart.lock names and leaves
# the lock alone, so a release PR ships the versions its lock was cut with.
#
# It refuses to run when Chart.lock has drifted from Chart.yaml, which is what a
# PR that edits only a dependency version (renovate) looks like — renovate moves
# Chart.yaml and never the lock. `update` is the only way forward there, but it
# re-resolves every dependency and rewrites the lock, so it is reserved for that
# one diagnosis; any other failure is fatal. Treating all build failures as
# drift is what let a missing repo definition silently re-resolve the lock on
# release PRs and ship an unintended Loki bump in v0.18.0.
#
# The versions in Chart.yaml are exact rather than `^` ranges, which bounds the
# damage `update` can do to the dependency the PR actually edited. That is a
# second, independent layer — do not lean on it to loosen this one.
function _install_deps() {
    local chart=$1
    local log
    log=$(mktemp)
    _info "==> helm dependency build $chart"
    if HELM_REPOSITORY_CONFIG="$REPO_CONFIG" "$HELM" dependency build "$chart" 2>&1 | tee "$log"; then
        rm -f "$log"
        return 0
    fi
    if grep -qF "out of sync" "$log"; then
        rm -f "$log"
        _warning "$chart: Chart.lock is out of sync with Chart.yaml."
        _info "==> helm dependency update $chart (re-resolves every dependency and rewrites Chart.lock)"
        HELM_REPOSITORY_CONFIG="$REPO_CONFIG" "$HELM" dependency update "$chart"
        return 0
    fi
    rm -f "$log"
    _error "$chart: 'helm dependency build' failed for a reason other than lock drift (see above)."
    _error "Refusing to fall back to 'helm dependency update': it would re-resolve every"
    _error "dependency and rewrite Chart.lock, shipping versions this commit never pinned."
    return 1
}

function _parse_args() {
    local arg
    CHARTS=()
    while [[ "$#" -gt 0 ]]; do
        arg="$1"
        shift
        case "$arg" in
            -h | --help)
                echo "Usage: $PROG CHART_DIR [CHART_DIR...]"
                echo "Populates each CHART_DIR/charts/ from its Chart.lock."
                exit 0
                ;;
            -*)
                _error "Unknown argument: $arg"
                echo "Usage: $PROG CHART_DIR [CHART_DIR...]"
                exit 1
                ;;
            *)
                CHARTS+=("$arg")
                ;;
        esac
    done
    if [ "${#CHARTS[@]}" -eq 0 ]; then
        _error "No chart directories given."
        echo "Usage: $PROG CHART_DIR [CHART_DIR...]"
        exit 1
    fi
}

function _main() {
    _parse_args "$@"
    if ! _has_prog yq; then
        _error "yq is required to read the chart dependencies. For example, brew install yq"
        return 1
    fi
    REPO_CONFIG=$(mktemp)
    # shellcheck disable=SC2064 # expand REPO_CONFIG now, not at trap time
    trap "rm -f '$REPO_CONFIG'" EXIT
    local chart
    for chart in "${CHARTS[@]}"; do
        if [ ! -f "$chart/Chart.yaml" ]; then
            _error "No Chart.yaml in $chart"
            return 1
        fi
        _register_repos "$chart"
        _install_deps "$chart"
    done
    _info "Subchart archives are in sync with Chart.lock."
}

_main "$@"
