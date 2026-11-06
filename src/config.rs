//! LingYan configuration, stored as JSON under %APPDATA%\LingYan.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub proxy: String,
    pub system_prompt: String,
    pub tts_rate: i32,
    /// Position: "corner" (screen corner) or "taskbar" (right end of taskbar).
    pub position: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".to_owned(),
            model: "glm-4.5-flash".to_owned(),
            proxy: String::new(),
            system_prompt: "你是运行在 Windows 桌面角落的小助手「灵言」。\
用户用语音和你说话，回答要简洁、口语化、中文，80 字以内，不要 Markdown。"
                .to_owned(),
            tts_rate: 1,
            position: "corner".to_owned(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("LingYan").join("config.json")
    }

    pub fn load_or_create() -> Self {
        let path = Self::path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<Self>(&text) {
                return config;
            }
        }
        let config = Self::default();
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path));
        if let Ok(text) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(&path, text);
        }
        config
    }

    pub fn open_in_notepad(&self) {
        let path = Self::path();
        let _ = std::process::Command::new("notepad.exe").arg(path).spawn();
    }
}
