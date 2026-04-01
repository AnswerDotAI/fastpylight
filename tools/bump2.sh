#!/bin/bash
set -e
cur=$(grep '^version = ' pyproject.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
new=$(echo "$cur" | awk -F. '{print $1"."$2+1"."0}')
sed -i '' "s/^version = \"$cur\"/version = \"$new\"/" pyproject.toml Cargo.toml
sed -i '' "s/^__version__ = '${cur}'/__version__ = '${new}'/" python/fastpylight/__init__.py
echo "$cur -> $new"
