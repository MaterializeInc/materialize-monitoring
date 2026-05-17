#!/usr/bin/env bash
# Check that git lfs is working properly

PROG=$0
# set cwd to repo root
cd "$(dirname "$0")/../" || exit 1
# shellcheck source=tools/shlib/common.sh
source "tools/shlib/common.sh"
set -o errexit -o errtrace -o nounset -o pipefail


AUTO_FIX=${AUTO_FIX:-false}

# Auto attempt to fix git lfs issues. This is used for CI or when --fix is manually passed.
function _attempt_install() {
    if _has_prog "brew"; then
        _info "Attempting to install Git LFS using Homebrew..."
        if brew install git-lfs; then
            _info "Git LFS installed successfully. Please run 'git lfs install' to initialize it for your user."
            return 0
        else
            _error "Failed to install Git LFS using Homebrew."
            return 1
        fi
    elif _has_prog "apt-get"; then
        # This path has a very low chance of success since we may need a specific repo to be added
        _info "Attempting to install Git LFS using apt-get..."
        if sudo apt-get update && sudo apt-get install git-lfs; then
            _info "Git LFS installed successfully. Please run 'git lfs install' to initialize it for your user."
            return 0
        else
            _error "Failed to install Git LFS using apt-get."
            return 1
        fi
    elif _has_prog "apk"; then
        _info "Attempting to install Git LFS using apk..."
        if apk add git-lfs; then
            _info "Git LFS installed successfully. Please run 'git lfs install' to initialize it for your user."
            return 0
        else
            _error "Failed to install Git LFS using apk."
            return 1
        fi
    else
        _error "No supported package manager found. Please install Git LFS manually from https://git-lfs.github.com/ and run 'git lfs install' to initialize it for your user."
        return 1
    fi
}

# Check that git lfs is installed and working properly
function _check_version() {
    _info "Checking Git LFS version"
    if ! git-lfs version; then
        if [ "$AUTO_FIX" = true ]; then
            _warning "Git LFS is not installed. Attempting to install it..."
            if _attempt_install; then
                git-lfs version
                return 0
            else
                _error "Auto-installation of Git LFS failed."
                return 1
            fi
        else
            _error "Git LFS is not installed. Please install it to work with this repository."
            _info "For example, brew install git-lfs"
            return 1
        fi
    fi
}

# Check that git lfs is initialized for this user/repo
function _check_initialized() {
    _info "Checking Git LFS initialization"
    if ! git config --get filter.lfs.smudge; then
        if [ "$AUTO_FIX" = true ]; then
            _warning "Git LFS is not initialized. Attempting to initialize it..."
            if git lfs install; then
                git config --get filter.lfs.smudge
                _info "Git LFS initialized successfully for this repository."
                return 0
            else
                _error "Failed to initialize Git LFS for this repository."
                return 1
            fi
        else
            _error "Git LFS is not initialized. Please run 'git lfs install' to initialize it for your user."
            return 1
        fi
    fi
}

# Ensure that all git lfs files are present.
# This will attempt to pull them regardless of --fix
function _check_files() {
    _info "Checking Git LFS files"
    if ! git lfs fsck --objects; then
        _warning "Some Git LFS files are missing. Attempting to pull them..."
        if ! git lfs pull; then
            _error "Failed to pull Git LFS files. Please check your network connection and try again."
            return 1
        fi
    fi
}

function _parse_args() {
    local arg
    while [[ "$#" -gt 0 ]]; do
        arg="$1"
        shift
        case "$arg" in
            --fix)
                AUTO_FIX=true
                ;;
            -h|--help)
                echo "Usage: $PROG [--fix]"
                echo "Checks that Git LFS is installed, initialized, and that all LFS files are present."
                exit 0
                ;;
            *)
                _error "Unknown argument: $arg"
                echo "Usage: $PROG [--fix]"
                echo "Checks that Git LFS is installed, initialized, and that all LFS files are present."
                exit 1
                ;;
        esac
    done
}

function _main() {
    _parse_args "$@"
    _check_version
    _check_initialized
    _check_files
    _info "Git LFS is working properly!"
}

_main "$@"
