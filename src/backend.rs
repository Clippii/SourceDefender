use serde::Deserialize;
use serde_json::{Value, json};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Run {
    pub id: String,
    pub folder: String,
    pub scenario: String,
    pub language: String,
    pub seed: String,
    pub started_utc: String,
    pub finished_utc: Option<String>,
    pub status: String,
    pub level: usize,
    pub penalty_seconds: i64,
    pub attempts: u32,
    pub hints: u32,
    pub broken: Vec<String>,
    pub events: Vec<String>,
    pub score: i64,
}
impl Run {
    pub fn seconds(&self) -> i64 {
        let start = chrono::DateTime::parse_from_rfc3339(&self.started_utc);
        let end = self
            .finished_utc
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|t| t.timestamp())
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        start
            .map(|t| (end - t.timestamp()).max(0) + self.penalty_seconds)
            .unwrap_or(0)
    }
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub mission: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Difficulty {
    pub name: String,
    pub bugs: u32,
    pub penalty: u32,
    pub hint_cost: u32,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Snapshot {
    pub runs: Vec<Run>,
    pub scenarios: Vec<Scenario>,
    pub difficulties: Vec<Difficulty>,
    pub languages: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Action {
    pub run: Run,
    pub report: String,
}

pub fn directory() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf()
}
pub fn call(request: Value) -> Result<Value, String> {
    call_in_store(request, None)
}
fn call_in_store(request: Value, store: Option<&std::path::Path>) -> Result<Value, String> {
    let name = if cfg!(windows) {
        "SourceDefender.Engine.exe"
    } else {
        "SourceDefender.Engine"
    };
    #[cfg(not(test))]
    let path = directory().join(name);
    #[cfg(test)]
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    let mut command = Command::new(&path);
    if let Some(path) = store {
        command.env("SOURCEDEFENDER_DATA_DIR", path);
    }
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("Cannot start the local game engine: {e}. Keep the engine beside SourceDefender or rebuild using the platform build script."))?;
    child
        .stdin
        .take()
        .ok_or("Engine input unavailable")?
        .write_all(request.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let reply: Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("Invalid engine reply: {e}"))?;
    if reply["ok"] != json!(true) {
        return Err(reply["error"]
            .as_str()
            .unwrap_or("Engine operation failed")
            .into());
    }
    if !output.status.success() {
        return Err("Engine exited unexpectedly".into());
    }
    Ok(reply["data"].clone())
}
pub fn time(seconds: i64) -> String {
    if seconds >= 3600 {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

pub fn default_folder() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(directory)
        .join("SourceDefender")
        .join("runs")
}

pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("The repository folder no longer exists.".into());
    }
    let mut command = Command::new(if cfg!(windows) {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    });
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    command
        .arg(path.canonicalize().map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| format!("Could not open folder: {e}"))?;
    Ok(())
}

#[cfg(windows)]
pub fn choose_folder() -> Result<Option<String>, String> {
    use windows_sys::Win32::{System::Com::*, UI::Shell::*};
    unsafe {
        let hr = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let title: Vec<u16> = "Choose a parent folder for the generated repository\0"
            .encode_utf16()
            .collect();
        let info = BROWSEINFOW {
            lpszTitle: title.as_ptr(),
            ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
            ..Default::default()
        };
        let item = SHBrowseForFolderW(&info);
        let mut path = [0u16; 260];
        let result = if !item.is_null() && SHGetPathFromIDListW(item, path.as_mut_ptr()) != 0 {
            Some(String::from_utf16_lossy(
                &path[..path.iter().position(|&x| x == 0).unwrap_or(path.len())],
            ))
        } else {
            None
        };
        if !item.is_null() {
            CoTaskMemFree(item.cast());
        }
        if hr >= 0 {
            CoUninitialize();
        }
        Ok(result)
    }
}

#[cfg(target_os = "macos")]
pub fn choose_folder() -> Result<Option<String>, String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            "POSIX path of (choose folder with prompt \"Choose a repository parent folder\")",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim_end().into(),
        ));
    }
    if String::from_utf8_lossy(&output.stderr).contains("-128") {
        return Ok(None);
    }
    Err("Folder selection failed. You can enter the folder path directly.".into())
}

#[cfg(target_os = "linux")]
pub fn choose_folder() -> Result<Option<String>, String> {
    for (name, args) in [
        (
            "zenity",
            vec![
                "--file-selection",
                "--directory",
                "--title=Choose a repository parent folder",
            ],
        ),
        ("kdialog", vec!["--getexistingdirectory", "."]),
    ] {
        match Command::new(name).args(args).output() {
            Ok(output) if output.status.success() => {
                return Ok(Some(
                    String::from_utf8_lossy(&output.stdout).trim_end().into(),
                ));
            }
            Ok(output) if output.status.code() == Some(1) => return Ok(None),
            _ => continue,
        }
    }
    Err("Install zenity or kdialog to browse folders, or enter a folder path directly.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timer_accounts_for_penalties_and_finished_time() {
        let run = Run {
            started_utc: "2026-10-28T12:00:00.0000000Z".into(),
            finished_utc: Some("2026-10-28T12:01:05.0000000Z".into()),
            penalty_seconds: 45,
            ..Default::default()
        };
        assert_eq!(run.seconds(), 110);
        assert_eq!(time(run.seconds()), "01:50");
        assert_eq!(time(3661), "01:01:01");
    }
    #[test]
    fn reads_existing_engine_run_format() {
        let text = r#"{"Id":"a","Folder":"C:\\runs","Scenario":"Midnight release","Language":"Python","Seed":"abc","StartedUtc":"2026-10-28T12:00:00.0000000Z","FinishedUtc":null,"Status":"Active","Level":0,"PenaltySeconds":0,"Attempts":0,"Hints":0,"Broken":["auth"],"Events":[],"Score":5000}"#;
        let run: Run = serde_json::from_str(text).unwrap();
        assert_eq!(run.status, "Active");
        assert_eq!(run.broken.len(), 1);
    }
    #[test]
    fn local_engine_round_trip() {
        let root =
            std::env::temp_dir().join(format!("source-defender-rust-test-{}", std::process::id()));
        let store = root.join("records");
        let snapshot = call_in_store(json!({"command":"snapshot"}), Some(&store)).unwrap();
        assert_eq!(snapshot["Scenarios"].as_array().unwrap().len(), 6);
        let action:Action=serde_json::from_value(call_in_store(json!({"command":"generate","level":0,"scenario":0,"language":"Python","seed":"unicode-λ","folder":root.join("répositories")}),Some(&store)).unwrap()).unwrap();
        let id = action.run.id;
        assert_eq!(action.run.seed, "unicode-λ");
        let bad: Action = serde_json::from_value(
            call_in_store(json!({"command":"validate","id":id}), Some(&store)).unwrap(),
        )
        .unwrap();
        assert_eq!(bad.run.penalty_seconds, 15);
        assert_eq!(bad.run.status, "Active");
        let intel: Action = serde_json::from_value(
            call_in_store(json!({"command":"hint","id":id}), Some(&store)).unwrap(),
        )
        .unwrap();
        assert_eq!(intel.run.hints, 1);
        assert_eq!(intel.run.penalty_seconds, 35);
        let ended: Action = serde_json::from_value(
            call_in_store(json!({"command":"end","id":id}), Some(&store)).unwrap(),
        )
        .unwrap();
        assert_eq!(ended.run.status, "Abandoned");
        assert!(ended.run.finished_utc.is_some());
        assert!(call_in_store(json!({"command":"validate","id":id}), Some(&store)).is_err());
        // Git objects are read-only on Windows; clear that attribute for our test tree.
        fn clean(path: &std::path::Path) {
            for entry in std::fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    clean(&path);
                } else {
                    #[cfg(windows)]
                    {
                        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
                        permissions.set_readonly(false);
                        let _ = std::fs::set_permissions(&path, permissions);
                    }
                }
            }
        }
        clean(&root);
        std::fs::remove_dir_all(root).unwrap();
    }
}
