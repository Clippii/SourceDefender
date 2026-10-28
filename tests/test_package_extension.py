"""Verify the release VSIX format without requiring native builds or VS Code."""

import importlib.util
import json
from pathlib import Path
import stat
import struct
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile


MODULE_PATH = Path(__file__).resolve().parents[1] / "tools" / "package_extension.py"
SPEC = importlib.util.spec_from_file_location("package_extension", MODULE_PATH)
PACKAGER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGER)
NS = {"v": PACKAGER.VSIX_NS, "c": PACKAGER.CONTENT_NS}


class PackageExtensionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        source = self.root / "ide-extension"
        (source / "resources").mkdir(parents=True)
        (source / "test").mkdir()
        self.package = {
            "name": "source-defender", "publisher": "Clippii", "version": "0.5.0",
            "main": "./extension.js", "displayName": "SourceDefender",
            "description": "Play & investigate", "engines": {"vscode": "^1.90.0"},
        }
        self.write_manifest()
        (source / "extension.js").write_text("module.exports = {};", encoding="utf-8")
        (source / "engine-client.js").write_text("module.exports = {};", encoding="utf-8")
        (source / "README.md").write_text("# SourceDefender", encoding="utf-8")
        (source / "resources" / "shield.svg").write_text("<svg/>", encoding="utf-8")
        (source / "test" / "extension.test.js").write_text("not runtime", encoding="utf-8")
        (source / "node_modules").mkdir()
        (source / "node_modules" / "secret.txt").write_text("not bundled", encoding="utf-8")

    def write_manifest(self):
        (self.root / "ide-extension" / "package.json").write_text(json.dumps(self.package), encoding="utf-8")

    def engine(self, rid):
        header = bytearray(80)
        name = "SourceDefender.Engine"
        if rid == "win-x64":
            name += ".exe"
            header[:2] = b"MZ"
            struct.pack_into("<I", header, 60, 64)
            header[64:70] = b"PE\0\0\x64\x86"
        elif rid == "linux-x64":
            header[:6] = b"\x7fELF\x02\x01"
            struct.pack_into("<H", header, 18, 62)
        else:
            header[:4] = b"\xcf\xfa\xed\xfe"
            struct.pack_into("<I", header, 4, 0x0100000C if rid == "osx-arm64" else 0x01000007)
        path = self.root / name
        path.write_bytes(header)
        return path

    def test_platform_packages_include_metadata_runtime_and_native_engine(self):
        for rid, target in PACKAGER.TARGETS.items():
            with self.subTest(rid=rid):
                engine = self.engine(rid)
                output = PACKAGER.build_vsix(self.root, rid, self.root / f"{rid}.vsix")
                with zipfile.ZipFile(output) as archive:
                    self.assertEqual(archive.testzip(), None)
                    manifest = ET.fromstring(archive.read("extension.vsixmanifest"))
                    identity = manifest.find("v:Metadata/v:Identity", NS)
                    self.assertEqual(identity.attrib["Id"], "source-defender")
                    self.assertEqual(identity.attrib["Publisher"], "Clippii")
                    self.assertEqual(identity.attrib["TargetPlatform"], target)
                    properties = {p.attrib["Id"]: p.attrib["Value"] for p in manifest.findall("v:Metadata/v:Properties/v:Property", NS)}
                    self.assertEqual(properties["Microsoft.VisualStudio.Code.Engine"], "^1.90.0")
                    for asset in manifest.findall("v:Assets/v:Asset", NS):
                        self.assertIn(asset.attrib["Path"], archive.namelist())
                    self.assertIn("extension/resources/shield.svg", archive.namelist())
                    self.assertIn("extension/engine-client.js", archive.namelist())
                    self.assertFalse(any("test/" in name or "node_modules/" in name for name in archive.namelist()))
                    bundled = archive.getinfo("extension/bin/" + engine.name)
                    self.assertEqual(archive.read(bundled), engine.read_bytes())
                    mode = bundled.external_attr >> 16
                    self.assertTrue(stat.S_ISREG(mode))
                    self.assertEqual(bool(mode & stat.S_IXUSR), rid != "win-x64")
                    ET.fromstring(archive.read("[Content_Types].xml"))

    def test_rejects_engine_for_the_wrong_platform(self):
        self.engine("linux-x64")
        with self.assertRaisesRegex(ValueError, "does not match osx-arm64"):
            PACKAGER.build_vsix(self.root, "osx-arm64", self.root / "wrong.vsix")

    def test_rejects_missing_runtime_and_unbundled_dependencies(self):
        self.engine("linux-x64")
        self.package["main"] = "missing.js"
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "packaged JavaScript"):
            PACKAGER.build_vsix(self.root, "linux-x64", self.root / "missing.vsix")
        self.package["dependencies"] = {"unbundled": "1.0.0"}
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "dependencies must be bundled"):
            PACKAGER.build_vsix(self.root, "linux-x64", self.root / "deps.vsix")


if __name__ == "__main__":
    unittest.main()
