#!/usr/bin/env bash
# Squash gh-pages to one parentless commit and drop PR previews that have not deployed recently.
#
# Usage: bin/squash-gh-pages.sh [--push]
#
# Without --push, the script builds the new commit and runs `git push --dry-run`,
# which checks the lease against the remote without changing anything.
#
# Every docs deploy and PR preview adds a commit of rendered HTML to gh-pages,
# so a full clone carries hundreds of MiB that the published site never reads.
# The new commit keeps the tip's tree, minus each preview/<slug> whose branch has no
# successful docs run in the last PREVIEW_MAX_AGE_DAYS days.
# Run times come from the GitHub API, because a squash erases the gh-pages history.
# To restore a dropped preview for an open PR, rerun its latest docs run or push to the PR.
#
# Only the tip's commit and trees are downloaded (--depth=1 --filter=blob:none), a few MiB.
# The push uses --force-with-lease against the tip the script read:
# if a deploy lands in between, the push fails instead of overwriting it, and you can rerun the script.
#
# The script refuses to run while a docs workflow run is queued or in progress.
# A deploy that starts after that check and pushes after the squash can still fail; rerun it.
#
# Environment:
#   REMOTE_URL            Repository to squash. Defaults to the URL of the origin remote.
#   BRANCH                Branch to squash. Defaults to gh-pages.
#   PREVIEW_MAX_AGE_DAYS  Drop previews with no successful docs run in this many days. Defaults to 2.

PROG=$0
# set cwd to repo root
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail

REMOTE_URL=${REMOTE_URL:-$(git remote get-url origin)}
BRANCH=${BRANCH:-gh-pages}
PREVIEW_MAX_AGE_DAYS=${PREVIEW_MAX_AGE_DAYS:-2}
PUSH=false

function _usage() {
    sed -n '2,/^$/s/^# \{0,1\}//p' "$PROG" >&2
}

# Same sanitization as the "Compute base URL and target folder" step in .github/workflows/docs.yaml.
function _slugify() {
    tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//'
}

# Print the UTC time PREVIEW_MAX_AGE_DAYS days ago in ISO 8601, with BSD or GNU date.
function _cutoff() {
    date -u -v-"${PREVIEW_MAX_AGE_DAYS}"d +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
        || date -u -d "${PREVIEW_MAX_AGE_DAYS} days ago" +%Y-%m-%dT%H:%M:%SZ
}

while [ $# -gt 0 ]; do
    case "$1" in
        --push) PUSH=true ;;
        -h | --help)
            _usage
            exit 0
            ;;
        *)
            _error "unknown argument: $1"
            _usage
            exit 2
            ;;
    esac
    shift
done

_require_progs git gh

active=$(gh run list --workflow docs.yaml --limit 50 --json status,displayTitle,url \
    --jq '.[] | select(.status != "completed") | "  \(.status)  \(.displayTitle)  \(.url)"')
if [ -n "$active" ]; then
    _error "docs workflow runs are still active; wait for them to finish:"
    _log "$active"
    exit 1
fi

cutoff=$(_cutoff)
branches=$(gh run list --workflow docs.yaml --event pull_request --status success \
    --created ">=$cutoff" --limit 1000 --json headBranch --jq '.[].headBranch')
# GitHub returns at most 1,000 runs for a filtered search, so a full page means older runs were cut off
# and their previews would be dropped as stale.
if [ "$(grep -c . <<<"$branches" || true)" -ge 1000 ]; then
    _error "more than 1,000 successful docs runs since $cutoff; lower PREVIEW_MAX_AGE_DAYS"
    exit 1
fi
fresh=$(_slugify <<<"$branches" | sort -u)

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT
repo="$workdir/repo.git"

_info "Fetching the tip of $BRANCH (commit and trees only) from $REMOTE_URL"
git clone --quiet --bare --single-branch --branch "$BRANCH" \
    --depth=1 --filter=blob:none "$REMOTE_URL" "$repo"

old=$(git -C "$repo" rev-parse "refs/heads/$BRANCH")
old_tree=$(git -C "$repo" rev-parse "$old^{tree}")

kept=()
pruned=()
if git -C "$repo" cat-file -e "$old:preview" 2>/dev/null; then
    while IFS=$'\t' read -r -d '' meta name; do
        if grep -qxF -- "$name" <<<"$fresh"; then
            kept+=("$meta"$'\t'"$name")
        else
            pruned+=("$name")
        fi
    done < <(git -C "$repo" ls-tree -z "$old:preview")
fi

# Rebuild the trees without the dropped previews.
# --missing lets mktree reference blobs that the blobless clone never downloaded.
new_tree=$old_tree
if [ ${#pruned[@]} -gt 0 ]; then
    preview_tree=
    if [ ${#kept[@]} -gt 0 ]; then
        preview_tree=$(printf '%s\0' "${kept[@]}" | git -C "$repo" mktree -z --missing)
    fi
    new_tree=$(
        git -C "$repo" ls-tree -z "$old_tree" | while IFS=$'\t' read -r -d '' meta name; do
            if [ "$name" != preview ]; then
                printf '%s\t%s\0' "$meta" "$name"
            elif [ -n "$preview_tree" ]; then
                printf '040000 tree %s\tpreview\0' "$preview_tree"
            fi
        done | git -C "$repo" mktree -z --missing
    )
fi

# A shallow clone hides the parents from rev-list, so read them from the raw commit.
old_commit=$(git -C "$repo" cat-file commit "$old")
if ! grep -q '^parent ' <<<"$old_commit" && [ "$new_tree" = "$old_tree" ]; then
    _info "$BRANCH is already a single commit with no stale previews ($old); nothing to do."
    exit 0
fi

message=(-m "Squash $BRANCH history" -m "Previous tip: $old. Made by bin/squash-gh-pages.sh.")
if [ ${#pruned[@]} -gt 0 ]; then
    _info "Dropping ${#pruned[@]} previews with no successful docs run since $cutoff:"
    printf '  preview/%s\n' "${pruned[@]}" >&2
    _info "Keeping ${#kept[@]} previews."
    message+=(-m "$(
        echo "Dropped previews with no successful docs run since $cutoff:"
        printf -- '- preview/%s\n' "${pruned[@]}"
    )")
fi

new=$(git -C "$repo" commit-tree "$new_tree" "${message[@]}")

_info "Old tip: $old (tree $old_tree)"
_info "New tip: $new (no parents, tree $new_tree)"

push_args=(--force-with-lease="refs/heads/$BRANCH:$old" "$REMOTE_URL" "$new:refs/heads/$BRANCH")
if [ "$PUSH" = true ]; then
    git -C "$repo" push "${push_args[@]}"
    _info "Squashed $BRANCH. Clones that still track it see a forced update."
else
    git -C "$repo" push --dry-run "${push_args[@]}"
    _info "Dry run only. Rerun with --push to replace $BRANCH."
fi
