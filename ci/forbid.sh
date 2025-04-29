#!/usr/bin/env bash

set -euo pipefail

if ! which rg > /dev/null; then
  echo 'error: `rg` not found, please install ripgrep: https://github.com/BurntSushi/ripgrep/'
  exit 1
fi

! rg \
  --glob !CHANGELOG.md \
  --glob !ci/forbid.sh \
  --ignore-case \
  'dbg!|fixme|todo|xxx' \
  .