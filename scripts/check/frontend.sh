#!/usr/bin/env bash

set -euo pipefail

bun typecheck
bun lint
bun test
bun run build
