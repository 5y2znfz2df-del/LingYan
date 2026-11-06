//! Minimal OpenAI-compatible chat client.

use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{json, Value};

use crate::config::Config;

pub fn chat(config: &Config, text: &str) -> Result<String, String> {
    if config.api_key.trim().is_empty() {
        return Err("请先编辑 config.json 填写 api_key".to_owned());
    }
    let base = config.base_url.trim().trim_end_matches('/');
    let url = format!("{base}/chat/completions");
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(40))
        .connect_timeout(Duration::from_secs(8));
    if !config.proxy.trim().is_empty() {
        builder = builder.proxy(
            reqwest::Proxy::all(config.proxy.trim()).map_err(|e| format!("代理地址无效：{e}"))?,
        );
    }
    let client = builder.build().map_err(|e| format!("创建网络客户端失败：{e}"))?;
    let body = json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": config.system_prompt},
            {"role": "user", "content": text}
        ],
        "temperature": 0.7,
        "max_tokens": 512,
        "stream": false
    });
    let response = client
        .post(&url)
        .bearer_auth(config.api_key.trim())
        .json(&body)
        .send()
        .map_err(|e| format!("网络请求失败：{e}"))?;
    let status = response.status();
    let text = response.text().map_err(|e| format!("读取响应失败：{e}"))?;
    if !status.is_success() {
        let short: String = text.chars().take(200).collect();
        return Err(match status.as_u16() {
            401 => "API Key 无效（401）".to_owned(),
            404 => format!("地址或模型不存在（404）：{short}"),
            429 => "请求太频繁或余额不足（429）".to_owned(),
            _ => format!("HTTP {}：{}", status.as_u16(), short),
        });
    }
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("响应不是 JSON：{e}"))?;
    let reply = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if reply.is_empty() {
        return Err("模型返回了空内容".to_owned());
    }
    Ok(reply)
}
