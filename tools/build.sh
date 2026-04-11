#!/bin/bash
set -euo pipefail

profile="${1:-debug}"

case "${profile}" in
  release)
    flags="--release"
    ;;
  debug)
    flags=""
    ;;
  *)
    echo "usage: $0 [debug|release]" >&2
    exit 1
    ;;
esac

python -m maturin build ${flags} --out dist -i python
