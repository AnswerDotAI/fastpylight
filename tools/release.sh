#!/bin/bash
set -e
v=$(grep '^version = ' pyproject.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
echo "Releasing v$v..."
git tag "v$v"
git push origin main --tags
echo "Released v$v — CI will build wheels and publish to PyPI"
