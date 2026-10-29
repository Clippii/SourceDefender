//! File and Git access for the local investigation workspace.
//!
//! This module only reads and saves source. Git is used for ordinary history and
//! diffs with external helpers disabled; project commands and hooks never run.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
use std::os::windows::{fs::MetadataExt, process::CommandExt};

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_FILES: usize = 3000;
const MAX_SEARCH_HITS: usize = 200;
const MAX_SEARCH_BYTES: usize = 16 * 1024 * 1024;
const MAX_GIT_BYTES: usize = 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

pub struct Repository {
    root: PathBuf,
    files: Vec<String>,
}

impl Repository {
    pub fn open(root: &Path) -> Result<Self, String> {
        let root = fs::canonicalize(root)
            .map_err(|e| format!("Could not open the incident folder: {e}"))?;
        if !root.is_dir() {
            return Err("The incident folder no longer exists.".into());
        }
        let mut repository = Self {
            root,
            files: Vec::new(),
        };
        repository.refresh()?;
        Ok(repository)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn files(&self) -> &[String] {
        &self.files
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        let mut files = Vec::new();
        let mut visited = 0;
        self.collect(&self.root, 0, &mut files, &mut visited)?;
        files.sort();
        self.files = files;
        Ok(())
    }

    fn collect(
        &self,
        directory: &Path,
        depth: usize,
        files: &mut Vec<String>,
        visited: &mut usize,
    ) -> Result<(), String> {
        if depth > 32 {
            return Err("This repository exceeds the workspace's 32-folder depth limit.".into());
        }
        let entries =
            fs::read_dir(directory).map_err(|e| format!("Could not list incident files: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Could not list incident files: {e}"))?;
            *visited += 1;
            if *visited > MAX_FILES * 3 {
                return Err(
                    "This repository has too many entries for the investigation workspace.".into(),
                );
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|e| format!("Could not inspect an incident file: {e}"))?;
            if is_link(&metadata) {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.eq_ignore_ascii_case(".git") {
                continue;
            }
            if metadata.is_dir() {
                if !skip_directory(&name) {
                    self.collect(&path, depth + 1, files, visited)?;
                }
            } else if metadata.is_file() {
                if files.len() >= MAX_FILES {
                    return Err(format!(
                        "This repository exceeds the workspace's {MAX_FILES}-file limit."
                    ));
                }
                if let Ok(relative) = path.strip_prefix(&self.root) {
                    files.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        Ok(())
    }

    fn resolve(&self, relative: &str) -> Result<PathBuf, String> {
        let normalized = relative.replace('\\', "/");
        if normalized.is_empty()
            || normalized.starts_with('/')
            || normalized.contains(':')
            || normalized.chars().any(char::is_control)
        {
            return Err("Choose a file inside the incident repository.".into());
        }
        let mut candidate = self.root.clone();
        for part in normalized.split('/') {
            if part.is_empty() || part == "." || part == ".." || part.eq_ignore_ascii_case(".git") {
                return Err("This path is outside the editable incident source.".into());
            }
            candidate.push(part);
            let metadata = fs::symlink_metadata(&candidate)
                .map_err(|e| format!("Could not open {relative}: {e}"))?;
            if is_link(&metadata) {
                return Err(
                    "Linked files and folders are excluded from the incident editor.".into(),
                );
            }
        }
        let actual =
            fs::canonicalize(&candidate).map_err(|e| format!("Could not open {relative}: {e}"))?;
        if !actual.starts_with(&self.root) || !actual.is_file() {
            return Err("Choose a regular file inside the incident repository.".into());
        }
        Ok(actual)
    }

    pub fn read(&self, relative: &str) -> Result<String, String> {
        let path = self.resolve(relative)?;
        read_text(&path).map_err(|e| format!("{relative}: {e}"))
    }

    /// Save only if the file still matches the version that the editor opened.
    /// A same-directory replacement avoids partial writes if saving is interrupted.
    pub fn save(&self, relative: &str, text: &str, expected_on_disk: &str) -> Result<(), String> {
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err("Files larger than 1 MiB cannot be saved in the incident editor.".into());
        }
        if text.contains('\0') {
            return Err("Binary content cannot be saved in the incident editor.".into());
        }
        let path = self.resolve(relative)?;
        if read_text(&path)? != expected_on_disk {
            return Err(format!(
                "{relative} changed on disk. Reload or review the external changes before saving; your editor changes are still here."
            ));
        }
        let permissions = fs::metadata(&path)
            .map_err(|e| e.to_string())?
            .permissions();
        if permissions.readonly() {
            return Err(format!(
                "{relative} is read-only. Your editor changes have not been discarded."
            ));
        }
        let parent = path
            .parent()
            .ok_or("The incident file has no parent folder.")?;
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".sourcedefender-save-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| format!("Could not prepare a safe save for {relative}: {e}"))?;
        let cleanup = TemporaryFile(temporary.clone());
        file.write_all(text.as_bytes())
            .map_err(|e| format!("Could not save {relative}: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Could not finish saving {relative}: {e}"))?;
        file.set_permissions(permissions)
            .map_err(|e| format!("Could not preserve file permissions: {e}"))?;
        drop(file);
        // Recheck after preparing the replacement so an ordinary external save
        // during the write does not silently replace the other editor's changes.
        if self.resolve(relative)? != path || read_text(&path)? != expected_on_disk {
            return Err(format!(
                "{relative} changed while saving. Reload or review the external changes before trying again."
            ));
        }
        fs::rename(&temporary, &path).map_err(|e| {
            format!("Could not replace {relative}: {e}. Your original file is unchanged.")
        })?;
        drop(cleanup);
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        let mut searched_bytes = 0;
        for relative in &self.files {
            let Ok(text) = self.read(relative) else {
                continue;
            };
            searched_bytes += text.len();
            if searched_bytes > MAX_SEARCH_BYTES {
                break;
            }
            for (index, line) in text.lines().enumerate() {
                if line.to_lowercase().contains(&query) {
                    hits.push(SearchHit {
                        path: relative.clone(),
                        line: index + 1,
                        text: line.chars().take(240).collect(),
                    });
                    if hits.len() >= MAX_SEARCH_HITS {
                        return hits;
                    }
                }
            }
        }
        hits
    }

    pub fn git_log(&self) -> Result<String, String> {
        self.git(&[
            "log",
            "-30",
            "--no-show-signature",
            "--no-decorate",
            "--date=short",
            "--format=%h  %ad  %s",
        ])
    }

    pub fn git_diff(&self, relative: Option<&str>) -> Result<String, String> {
        let mut arguments = vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--color=never",
            "--unified=3",
            "--",
        ];
        if let Some(relative) = relative {
            // File selection must follow exactly the same boundary as editing.
            self.resolve(relative)?;
            arguments.push(relative);
        }
        self.git(&arguments)
    }

    fn git(&self, arguments: &[&str]) -> Result<String, String> {
        let mut command = Command::new("git");
        command
            .current_dir(&self.root)
            .args([
                "--no-pager",
                "--no-optional-locks",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "diff.external=",
                "-c",
                "color.ui=false",
                "-c",
                "log.showSignature=false",
            ])
            .args(arguments)
            .env_remove("GIT_EXTERNAL_DIFF")
            .env_remove("GIT_DIFF_OPTS")
            .env_remove("GIT_CONFIG_PARAMETERS")
            .env_remove("GIT_CONFIG_COUNT")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child = command.spawn().map_err(|e| {
            format!("Git history is unavailable: {e}. Source editing and validation still work.")
        })?;
        let stdout = child.stdout.take().ok_or("Could not read Git output.")?;
        let stderr = child.stderr.take().ok_or("Could not read Git errors.")?;
        let output_reader = thread::spawn(move || bounded_output(stdout, MAX_GIT_BYTES));
        let error_reader = thread::spawn(move || bounded_output(stderr, 8192));
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if started.elapsed() < Duration::from_secs(5) => {
                    thread::sleep(Duration::from_millis(20))
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(
                        "Git took too long. Source editing and validation still work.".to_string(),
                    );
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(format!("Could not read Git's result: {error}"));
                }
            }
        };
        let (output, truncated) = output_reader
            .join()
            .map_err(|_| "Could not read Git output.")??;
        let (errors, _) = error_reader
            .join()
            .map_err(|_| "Could not read Git errors.")??;
        let status = status?;
        if !status.success() {
            return Err(format!(
                "Git could not read this repository: {}",
                String::from_utf8_lossy(&errors).trim()
            ));
        }
        let mut result = String::from_utf8_lossy(&output).into_owned();
        if truncated {
            result.push_str(
                "\n\n[Output limited to 1 MiB. Select an individual file to narrow the diff.]",
            );
        }
        Ok(result)
    }
}

fn skip_directory(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "node_modules" | "target" | "bin" | "obj" | "__pycache__" | "vendor" | "dist" | ".build"
    )
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    if metadata.file_attributes() & 0x400 != 0 {
        return true;
    }
    metadata.file_type().is_symlink()
}

fn read_text(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("Could not read this file: {e}"))?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
        return Err("This file is larger than the editor's 1 MiB limit.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("This file is larger than the editor's 1 MiB limit.".into());
    }
    if bytes.contains(&0) {
        return Err("This is a binary file. Choose a source or configuration file.".into());
    }
    String::from_utf8(bytes)
        .map_err(|_| "This file is not UTF-8 text and cannot be edited here.".into())
}

fn bounded_output(mut reader: impl Read, maximum: usize) -> Result<(Vec<u8>, bool), String> {
    let mut result = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        let remaining = maximum.saturating_sub(result.len());
        result.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    Ok((result, truncated))
}

struct TemporaryFile(PathBuf);
impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFolder(PathBuf);
    impl TestFolder {
        fn new() -> Self {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "source-defender-workspace-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn write(&self, relative: &str, text: &str) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
    }
    impl Drop for TestFolder {
        fn drop(&mut self) {
            fn clean(path: &Path) {
                let Ok(entries) = fs::read_dir(path) else {
                    return;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    let Ok(metadata) = fs::symlink_metadata(&path) else {
                        continue;
                    };
                    if is_link(&metadata) {
                        continue;
                    }
                    if metadata.is_dir() {
                        clean(&path);
                    }
                    #[cfg(windows)]
                    if metadata.permissions().readonly() {
                        let mut permissions = metadata.permissions();
                        permissions.set_readonly(false);
                        let _ = fs::set_permissions(&path, permissions);
                    }
                }
            }
            clean(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn explorer_keeps_hidden_evidence_and_ignores_build_folders() {
        let folder = TestFolder::new();
        folder.write(".github/workflows/release.yml", "build");
        folder.write(".gitignore", "src/\n");
        folder.write("src/main.py", "print('safe')\n");
        folder.write(".git/config", "private");
        folder.write("node_modules/ignored.js", "generated");
        folder.write("target/ignored.rs", "generated");
        let repository = Repository::open(&folder.0).unwrap();
        assert_eq!(
            repository.files(),
            &[".github/workflows/release.yml", ".gitignore", "src/main.py"]
        );
        assert_eq!(repository.read("src\\main.py").unwrap(), "print('safe')\n");
    }

    #[test]
    fn save_preserves_exact_text_and_rejects_external_changes() {
        let folder = TestFolder::new();
        folder.write("src/évidence.py", "before\r\n");
        let repository = Repository::open(&folder.0).unwrap();
        repository
            .save("src/évidence.py", "repaired λ\r\n", "before\r\n")
            .unwrap();
        assert_eq!(
            repository.read("src/évidence.py").unwrap(),
            "repaired λ\r\n"
        );
        fs::write(folder.0.join("src/évidence.py"), "external edit\n").unwrap();
        assert!(
            repository
                .save("src/évidence.py", "my unsaved edit\n", "repaired λ\r\n")
                .unwrap_err()
                .contains("changed on disk")
        );
        assert_eq!(
            repository.read("src/évidence.py").unwrap(),
            "external edit\n"
        );
        assert!(fs::read_dir(folder.0.join("src")).unwrap().all(|e| {
            !e.unwrap()
                .file_name()
                .to_string_lossy()
                .contains("sourcedefender-save")
        }));
    }

    #[test]
    fn editor_rejects_metadata_traversal_binary_and_oversize_content() {
        let folder = TestFolder::new();
        folder.write("source.txt", "safe");
        folder.write(".git/config", "private");
        fs::write(folder.0.join("binary"), [1, 0, 2]).unwrap();
        fs::write(
            folder.0.join("large.txt"),
            vec![b'x'; MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        let repository = Repository::open(&folder.0).unwrap();
        for path in [
            "../source.txt",
            "./source.txt",
            "/source.txt",
            "C:\\source.txt",
            "source.txt:stream",
            ".git/config",
            ".GIT/config",
            "src/../source.txt",
            "",
        ] {
            assert!(repository.read(path).is_err(), "accepted {path}");
            assert!(
                repository.save(path, "edit", "safe").is_err(),
                "saved {path}"
            );
        }
        assert!(repository.read("binary").is_err());
        assert!(repository.read("large.txt").is_err());
        assert!(
            repository
                .save(
                    "source.txt",
                    &"x".repeat(MAX_FILE_BYTES as usize + 1),
                    "safe"
                )
                .is_err()
        );
        assert_eq!(repository.read("source.txt").unwrap(), "safe");
    }

    #[test]
    fn source_search_is_case_insensitive_bounded_and_uses_one_based_lines() {
        let folder = TestFolder::new();
        folder.write("README.md", "briefing\nEvidence λ\n");
        folder.write("src/main.py", &"eVIDence\n".repeat(MAX_SEARCH_HITS + 20));
        let repository = Repository::open(&folder.0).unwrap();
        let results = repository.search("evidence");
        assert_eq!(results.len(), MAX_SEARCH_HITS);
        assert_eq!(
            results[0],
            SearchHit {
                path: "README.md".into(),
                line: 2,
                text: "Evidence λ".into()
            }
        );
        assert_eq!(results[1].line, 1);
        assert!(repository.search("   ").is_empty());
    }

    #[test]
    fn linked_entries_cannot_escape_or_cycle_through_the_editor() {
        let folder = TestFolder::new();
        let outside = TestFolder::new();
        outside.write("secret.txt", "outside");
        folder.write("source.txt", "inside");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside.0, folder.0.join("escape")).unwrap();
            std::os::unix::fs::symlink(&folder.0, folder.0.join("cycle")).unwrap();
            std::os::unix::fs::symlink(outside.0.join("secret.txt"), folder.0.join("file-link"))
                .unwrap();
        }
        #[cfg(windows)]
        {
            for (name, target) in [("escape", &outside.0), ("cycle", &folder.0)] {
                let output = Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(folder.0.join(name))
                    .arg(target)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        let repository = Repository::open(&folder.0).unwrap();
        assert_eq!(repository.files(), &["source.txt"]);
        assert!(repository.read("escape/secret.txt").is_err());
        assert!(
            repository
                .save("escape/secret.txt", "overwritten", "outside")
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(outside.0.join("secret.txt")).unwrap(),
            "outside"
        );
        assert!(repository.read("cycle/source.txt").is_err());
        #[cfg(unix)]
        assert!(repository.read("file-link").is_err());
    }

    #[test]
    fn git_views_show_history_and_working_changes_without_external_helpers() {
        if Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .status()
            .is_err()
        {
            return;
        }
        let folder = TestFolder::new();
        folder.write("src/main.py", "before\n");
        for args in [
            vec!["init", "-q"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=SourceDefender test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "Trusted source",
            ],
        ] {
            assert!(
                Command::new("git")
                    .current_dir(&folder.0)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        folder.write("src/main.py", "after\n");
        let repository = Repository::open(&folder.0).unwrap();
        assert!(repository.git_log().unwrap().contains("Trusted source"));
        let diff = repository.git_diff(Some("src/main.py")).unwrap();
        assert!(
            diff.contains("-before") && diff.contains("+after"),
            "{diff}"
        );
        assert!(repository.git_diff(Some(".git/config")).is_err());
        assert!(
            Command::new("git")
                .current_dir(&folder.0)
                .args([
                    "config",
                    "diff.external",
                    "sourcedefender-this-helper-must-not-run"
                ])
                .status()
                .unwrap()
                .success()
        );
        assert!(repository.git_diff(None).unwrap().contains("+after"));
    }
}
