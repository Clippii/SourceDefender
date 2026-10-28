#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) rid=linux-x64 ;;
  Linux-aarch64|Linux-arm64) rid=linux-arm64 ;;
  Darwin-arm64) rid=osx-arm64 ;;
  Darwin-x86_64) rid=osx-x64 ;;
  *) echo 'Unsupported OS/CPU combination' >&2; exit 1 ;;
esac
dotnet publish engine/SourceDefender.Engine.csproj -c Release -r "$rid" \
  --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true \
  -p:EnableCompressionInSingleFile=true -o ".build/engine/$rid" --nologo
cp ".build/engine/$rid/SourceDefender.Engine" ./SourceDefender.Engine
chmod +x SourceDefender.Engine
cargo build --release --locked
mkdir -p "dist/$rid"
if [[ "$rid" == osx-* ]]; then
  app="dist/$rid/SourceDefender.app/Contents"
  mkdir -p "$app/MacOS" "$app/Resources"
  cp target/release/SourceDefender SourceDefender.Engine "$app/MacOS/"
  cp assets/Info.plist "$app/"
  if [[ -f assets/icon.png ]]; then
    mkdir -p .build/icon.iconset
    for size in 16 32 128 256 512; do
      sips -z "$size" "$size" assets/icon.png --out ".build/icon.iconset/icon_${size}x${size}.png" >/dev/null
      double=$((size * 2))
      sips -z "$double" "$double" assets/icon.png --out ".build/icon.iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil -c icns .build/icon.iconset -o "$app/Resources/SourceDefender.icns"
  fi
  # Ad-hoc signing permits local execution; the release is not Apple-notarized.
  codesign --force --deep --sign - "dist/$rid/SourceDefender.app"
else
  cp target/release/SourceDefender SourceDefender.Engine "dist/$rid/"
  cp assets/SourceDefender.desktop "dist/$rid/"
  cp assets/icon.png "dist/$rid/"
fi
cp README.md "dist/$rid/"
cp docs/MCP.md "dist/$rid/"
mkdir -p "dist/$rid/docs"
cp docs/MCP.md "dist/$rid/docs/"
if [[ "${1:-}" == --test ]]; then
  cargo test --release --locked
  ./SourceDefender.Engine --self-test
  cat test-results.txt
fi
echo "Build available in dist/$rid"
