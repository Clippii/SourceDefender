#!/usr/bin/env python3
"""Build a platform-specific SourceDefender VSIX using the Python standard library."""

from __future__ import annotations

import argparse
import json
import mimetypes
from pathlib import Path, PurePosixPath
import stat
import struct
import sys
import xml.etree.ElementTree as ET
import zipfile


TARGETS = {
    "win-x64": "win32-x64",
    "linux-x64": "linux-x64",
    "osx-arm64": "darwin-arm64",
    "osx-x64": "darwin-x64",
}
VSIX_NS = "http://schemas.microsoft.com/developer/vsx-schema/2011"
CONTENT_NS = "http://schemas.openxmlformats.org/package/2006/content-types"


def _xml(element: ET.Element) -> bytes:
    return ET.tostring(element, encoding="utf-8", xml_declaration=True)


def _check_engine(engine: Path, rid: str) -> None:
    """Catch an accidentally reused executable from a different build platform."""
    with engine.open("rb") as stream:
        header = stream.read(64)
        if rid == "win-x64" and header[:2] == b"MZ" and len(header) == 64:
            stream.seek(struct.unpack_from("<I", header, 60)[0])
            pe_header = stream.read(6)
            if pe_header[:4] == b"PE\0\0" and pe_header[4:] == b"\x64\x86":
                return
        elif rid == "linux-x64" and header[:6] == b"\x7fELF\x02\x01":
            if len(header) >= 20 and struct.unpack_from("<H", header, 18)[0] == 62:
                return
        elif rid.startswith("osx-") and len(header) >= 8:
            endian = {b"\xcf\xfa\xed\xfe": "<", b"\xfe\xed\xfa\xcf": ">"}.get(header[:4])
            expected_cpu = 0x0100000C if rid == "osx-arm64" else 0x01000007
            if endian and struct.unpack_from(endian + "I", header, 4)[0] == expected_cpu:
                return
    raise ValueError(f"Engine does not match {rid}: {engine}")


def _manifest(package: dict, target: str, files: dict[str, Path]) -> bytes:
    manifest = ET.Element("PackageManifest", {"Version": "2.0.0", "xmlns": VSIX_NS})
    metadata = ET.SubElement(manifest, "Metadata")
    ET.SubElement(metadata, "Identity", {
        "Language": "en-US", "Id": package["name"], "Version": package["version"],
        "Publisher": package["publisher"], "TargetPlatform": target,
    })
    ET.SubElement(metadata, "DisplayName").text = package.get("displayName", package["name"])
    ET.SubElement(metadata, "Description", {"{http://www.w3.org/XML/1998/namespace}space": "preserve"}).text = package.get("description", "")
    ET.SubElement(metadata, "Tags").text = ",".join(package.get("keywords", []))
    ET.SubElement(metadata, "Categories").text = ",".join(package.get("categories", ["Other"]))
    properties = ET.SubElement(metadata, "Properties")
    for key, value in {
        "Microsoft.VisualStudio.Code.Engine": package["engines"]["vscode"],
        "Microsoft.VisualStudio.Code.ExtensionDependencies": ",".join(package.get("extensionDependencies", [])),
        "Microsoft.VisualStudio.Code.ExtensionPack": ",".join(package.get("extensionPack", [])),
        "Microsoft.VisualStudio.Code.ExtensionKind": ",".join(package.get("extensionKind", ["workspace"])),
        "Microsoft.VisualStudio.Code.ExecutesCode": "true",
        "Microsoft.VisualStudio.Code.WebExtension": "false",
        "Microsoft.VisualStudio.Code.PreRelease": "false",
    }.items():
        ET.SubElement(properties, "Property", {"Id": key, "Value": value})
    if "extension/LICENSE" in files:
        ET.SubElement(metadata, "License").text = "extension/LICENSE"
    icon = package.get("icon")
    if icon:
        ET.SubElement(metadata, "Icon").text = "extension/" + icon
    installation = ET.SubElement(manifest, "Installation")
    ET.SubElement(installation, "InstallationTarget", {"Id": "Microsoft.VisualStudio.Code"})
    ET.SubElement(manifest, "Dependencies")
    assets = ET.SubElement(manifest, "Assets")
    for asset_type, path in [
        ("Microsoft.VisualStudio.Code.Manifest", "extension/package.json"),
        ("Microsoft.VisualStudio.Services.Content.Details", "extension/README.md"),
    ] + ([("Microsoft.VisualStudio.Services.Icons.Default", "extension/" + icon)] if icon else []):
        ET.SubElement(assets, "Asset", {"Type": asset_type, "Path": path, "Addressable": "true"})
    return _xml(manifest)


def _content_types(paths: list[str]) -> bytes:
    types = ET.Element("Types", {"xmlns": CONTENT_NS})
    defaults = {"json": "application/json", "js": "application/javascript", "md": "text/markdown", "vsixmanifest": "text/xml"}
    for path in sorted(paths):
        suffix = PurePosixPath(path).suffix.lstrip(".")
        if suffix:
            defaults.setdefault(suffix, mimetypes.guess_type(path)[0] or "application/octet-stream")
        else:
            ET.SubElement(types, "Override", {"PartName": "/" + path, "ContentType": "application/octet-stream"})
    for suffix, content_type in sorted(defaults.items()):
        ET.SubElement(types, "Default", {"Extension": suffix, "ContentType": content_type})
    return _xml(types)


def build_vsix(root: Path, rid: str, output: Path, engine: Path | None = None) -> Path:
    root = root.resolve()
    source = root / "ide-extension"
    package = json.loads((source / "package.json").read_text(encoding="utf-8"))
    for key in ("name", "publisher", "version", "main"):
        if not isinstance(package.get(key), str) or not package[key]:
            raise ValueError(f"package.json requires a nonempty {key}")
    if not isinstance(package.get("engines", {}).get("vscode"), str):
        raise ValueError("package.json requires engines.vscode")
    if package.get("dependencies"):
        raise ValueError("Runtime dependencies must be bundled before creating the VSIX")

    engine_name = "SourceDefender.Engine.exe" if rid == "win-x64" else "SourceDefender.Engine"
    engine = engine or root / engine_name
    _check_engine(engine, rid)
    files = {"extension/package.json": source / "package.json", "extension/README.md": source / "README.md"}
    for path in sorted(source.glob("*.js")):
        files["extension/" + path.name] = path
    resources = source / "resources"
    if resources.exists():
        for path in sorted(resources.rglob("*")):
            if path.is_file():
                files["extension/" + path.relative_to(source).as_posix()] = path
    if (source / "LICENSE").is_file():
        files["extension/LICENSE"] = source / "LICENSE"
    elif (root / "LICENSE").is_file():
        files["extension/LICENSE"] = root / "LICENSE"
    files["extension/bin/" + engine_name] = engine

    main = PurePosixPath(package["main"])
    if main.is_absolute() or ".." in main.parts or "extension/" + str(main) not in files:
        raise ValueError("package.json main must identify a packaged JavaScript runtime file")
    icon = package.get("icon")
    if icon and "extension/" + icon not in files:
        raise ValueError(f"The extension icon is missing from the package: {icon}")
    for archive_path, source_path in files.items():
        if not source_path.is_file():
            raise ValueError(f"Required extension file is missing: {source_path}")
        if source_path.is_symlink():
            raise ValueError(f"Extension files must not be symbolic links: {source_path}")

    metadata = {
        "extension.vsixmanifest": _manifest(package, TARGETS[rid], files),
        "[Content_Types].xml": _content_types(list(files) + ["extension.vsixmanifest"]),
    }
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output in {path.resolve() for path in files.values()}:
        raise ValueError("VSIX output cannot overwrite an extension source file")
    try:
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
            for name in sorted(set(metadata) | set(files)):
                info = zipfile.ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
                info.create_system = 3
                mode = 0o755 if name == "extension/bin/" + engine_name and rid != "win-x64" else 0o644
                info.external_attr = (stat.S_IFREG | mode) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(info, metadata[name] if name in metadata else files[name].read_bytes())
    except BaseException:
        output.unlink(missing_ok=True)
        raise
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rid", required=True, choices=TARGETS)
    parser.add_argument("--output", type=Path, help="VSIX destination (default: SourceDefender-<rid>.vsix)")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1], help="SourceDefender repository")
    parser.add_argument("--engine", type=Path, help="Override the already published self-contained engine")
    args = parser.parse_args()
    try:
        result = build_vsix(args.root, args.rid, args.output or Path(f"SourceDefender-{args.rid}.vsix"), args.engine)
    except (OSError, ValueError, KeyError) as error:
        print(f"Cannot package extension: {error}", file=sys.stderr)
        return 1
    print(f"Packaged {result.name} for {TARGETS[args.rid]} ({result.stat().st_size:,} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
