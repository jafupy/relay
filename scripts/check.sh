#!/usr/bin/env bash

set -euo pipefail

bash scripts/check/frontend.sh
bash scripts/check/rust.sh
