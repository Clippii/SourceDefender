SourceDefender is a local blue-team game about investigating attacked source repositories.

- Native Rust desktop interface with a quieter layout, sidebar navigation, and clearer run status.
- Windows x64, Linux x64, macOS Apple Silicon, and macOS Intel builds.
- Self-contained local engine; no .NET installation or online service needed to play.
- Six scenarios, four source languages, sixteen attack types, and four difficulty levels.
- Extensive generated projects with business logic, tests, fixtures, Git evidence, and layered obfuscation.
- Timed investigations, validation penalties, hints, saved runs, and local history.

Extract the download for your platform and keep the application and engine together. On macOS, extract and move SourceDefender.app to Applications. Mac builds are ad-hoc signed, not Apple notarized; macOS may require approval through Privacy & Security for the first launch. Linux needs a graphical desktop with OpenGL; folder browsing uses zenity or kdialog (typing a path always works).

All four release builds must pass engine tests, Rust integration tests, generated-project regression tests, and a native-window smoke test before this release is published. SHA256SUMS.txt contains checksums for the downloads.
