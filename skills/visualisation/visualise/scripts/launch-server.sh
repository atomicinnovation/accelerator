#!/usr/bin/env bash
set -euo pipefail

# Placeholder sentinel for the meta visualiser launcher.
# Prints a deliberately-invalid URL scheme so a user who pastes this into
# a browser sees an immediate "invalid URL" signal rather than a
# connection-refused error that suggests a real server failed to start.
# The real Rust server bootstrap lands in a later phase and replaces this
# file wholesale.

echo "placeholder://phase-1-scaffold-not-yet-running"
