#!/usr/bin/env bash
exec "${ACCELERATOR_BIN:?ACCELERATOR_BIN must point at the compiled accelerator launcher}" config get "$@"
