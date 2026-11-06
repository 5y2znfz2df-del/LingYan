//! Offline Mandarin recognition through the Windows SAPI recognizer.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::Duration;

const SCRIPT: &str = r#"
param([string]$Wave)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$r = New-Object System.Speech.Recognition.SpeechRecognitionEngine([System.Globalization.CultureInfo]::GetCultureInfo('zh-CN'))
$r.LoadGrammar((New-Object System.Speech.Recognition.DictationGrammar))
$r.SetInputToWaveFile($Wave)
$parts = New-Object System.Collections.Generic.List[string]
while ($true) {
    $res = $r.Recognize()
    if ($null -eq $res) { break }
    if ($res.Text) { $parts.Add($res.Text) }
}
$r.Dispose()
[Console]::Out.Write(($parts -join ''))
"#;

fn wav_bytes(pcm: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in pcm {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

pub fn recognize(pcm: &[i16]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let wav = wav_bytes(pcm, 16_000);
    let dir = std::env::temp_dir();
    let wav_path = dir.join("lingyan_zh.wav");
    let script_path = dir.join("lingyan_zh.ps1");
    std::fs::write(&wav_path, wav).map_err(|error| format!("写入录音失败：{error}"))?;
    std::fs::write(&script_path, SCRIPT).map_err(|error| format!("写入识别脚本失败：{error}"))?;

    let mut child = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .arg("-Wave")
        .arg(&wav_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("启动 Windows 中文识别失败：{error}"))?;

    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() > Duration::from_secs(12) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Windows 中文识别超时".to_owned());
                }
                std::thread::sleep(Duration::from_millis(30));
            }
            Err(error) => return Err(format!("Windows 中文识别等待失败：{error}")),
        }
    }

    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    let text = stdout.trim().to_owned();
    if text.is_empty() {
        let detail = stderr.trim();
        return Err(if detail.is_empty() {
            "没有听清，请再说一次".to_owned()
        } else {
            format!("没有听清：{detail}")
        });
    }
    Ok(text)
}
