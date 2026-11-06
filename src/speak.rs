//! Speak text through Windows SAPI (offline, no extra install).

use std::io::Write;
use std::process::{Command, Stdio};

pub fn speak(text: &str, rate: i32) {
    if text.trim().is_empty() {
        return;
    }
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let dir = std::env::temp_dir();
    let path = dir.join("lingyan_speak.txt");
    if std::fs::write(&path, text).is_err() {
        return;
    }
    let script = format!(
        "Add-Type -AssemblyName System.Speech; \
         $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         $s.Rate = {rate}; \
         $t = [IO.File]::ReadAllText($env:LINGYAN_TEXT, [Text.Encoding]::UTF8); \
         $s.Speak($t); $s.Dispose()"
    );
    let _ = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .env("LINGYAN_TEXT", &path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .and_then(|mut child| child.wait());
    let _ = std::io::stdout().flush();
}
