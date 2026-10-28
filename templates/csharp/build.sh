#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")"
dotnet build Service.csproj -c Release --nologo
if [ "${1:-}" = "--test" ]; then
    dotnet run --project Service.csproj -c Release --no-build -- --test
fi
