#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")"
if [ ! -x ./SourceDefender ] || [ ! -x ./SourceDefender.Engine ]; then
  bash ./build.sh
  cp target/release/SourceDefender ./SourceDefender
fi
exec ./SourceDefender
