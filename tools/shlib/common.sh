#!/usr/bin/env bash
# Common functions for bash scripts
#
# Usage::
#   # shellcheck source=tools/shlib/common.sh
#   source "$(dirname "$0")/tools/shlib/common.sh"
#   set -o errexit -o errtrace -o nounset -o pipefail
#
# Author: Heather Lapointe


# Writes a message to stderr
# $* is the message to be echo'd.
function _log() {
    # >&2 means that the default (stdout) is being redirected (>) to &2 (stderr)
    echo -e "$@" >&2
}

# Write a message to stderr with a light blue foreground.
#   $* - Message
function _info() {
    _log "\\033[0;36m$*\\033[0m"
}

# Write a message to stderr with Yellow foreground. WARNING: will be prepended with a red background.
#   $* - Message
function _warning() {
    _log "\\033[1;33m\\033[41mWARNING\\033[49m: $*\\033[0m"
}

# Write a message to stderr using Red foreground.
# It will be prefixed with ERROR: automatically.
#   $* - Message
function _error() {
    _log "\\033[1;91mERROR: $*\\033[0m"
}

## _has_prog: Check if a program/function exists
# Usage: if ! _has_prog; then ...
function _has_prog() {
    command -v "$1" >/dev/null
}

# Callback function for when set -e (ERREXIT) is triggered)
# This shows a small stacktrace for the current shell along with the failing
# function call.
function _show_traceback() {
    # The very first line captures the return code in $?.
    local rc=$?
    # We pass "${BASH_SOURCE[0]}" and $LINENO (note: not $BASH_LINENO) from
    # the trap so they refer to the failing call before this is run.
    # Otherwise, they refer to this _show_traceback itself.
    local fail_file=$1
    local file_line=$2
    # traceback locals
    local line_idx;
    local filename;
    # FUNCNAME is an array of functions
    local frame=${#FUNCNAME[@]}
    # skip "main" frame
    frame=$(( frame - 1 ))

    _warning "Subcommand failed with code=$rc. Bash traceback:"
    # Roughly an enumeration like::
    #   for frame_id, func in reversed(enumerate(FUNCNAME[1:])):
    #       filename = BASH_SOURCE[frame_id+1]
    #       line_idx = BASH_LINENO[frame_id]
    while [ "$frame" -gt 1 ]; do
        # BASH_SOURCE is +1 from BASH_LINENO and FUNCNAME index
        filename="${BASH_SOURCE[$frame]:-"<script>"}"
        # decrement after getting source, which is +1 of the rest
        frame=$(( frame - 1 ))
        line_idx="${BASH_LINENO[$frame]}"
        func="${FUNCNAME[$frame]}"
        _log "In $filename, line $line_idx:"
        _log "[#$frame]\t$func"
    done
    # frame 0 is _show_traceback, so we instead show $1 and $2 from our trap
    _log "In $fail_file, line $file_line:"
    _error "\t$BASH_COMMAND"
    _error "\t^-- returned $rc"
    _log ""
    return "$rc"
}

function _register_traceback() {
    # Cause shell to exit whenever any command fails.
    # Allows the ERR trap to be called before exit. (errexit)
    set -e
    # Ensure _show_traceback is propagated through functions (errtrace)
    set -E

    if [[ "${SHELL}" != *bash* ]]; then
        _error "The current trace handler only works in bash. Current shell: ${SHELL}"
        exit 1
    fi
    # Invoke _show_traceback whenever a failure occurs (via set -e)
    # Pass the current script and lineno (man (1) bash for LINENO usage)
    trap '_show_traceback "${BASH_SOURCE[0]}" "$LINENO"' ERR
}
