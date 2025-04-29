#!/usr/bin/env bash

set -euxo pipefail

VERSION=${REF#"refs/tags/"}
docker buildx build --platform linux/amd64,linux/arm64 \
  -t slipwayhq/slipway:$VERSION \
  -t slipwayhq/slipway:latest \
  . --push
