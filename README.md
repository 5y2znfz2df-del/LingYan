# 灵言 LingYan

Siri 风格的 Windows 语音小部件：屏幕角落一颗黑色液态玻璃光球（v0.1 固定在左上角），点一下说话，
中文识别 → 大模型回答 → 朗读，并在光球上方弹出小面板显示文字。

不依赖 WinIsland 灵动岛，可配合 **Windhawk** 随 explorer 自动启动。

> v0.1：屏幕左上角模式已可用；右下角/任务栏内嵌图标是下一步。

## 功能

- Siri 液态玻璃光球（黑玻璃渐变 + 边缘彩虹色散 + 左上高光）
- 点击说话：自动录音、1.5 秒静音结束
- 中文识别：Windows SAPI 离线识别（无需下载模型）
- 任意 OpenAI 兼容大模型（默认智谱 `glm-4.5-flash`）
- 朗读回复：Windows SAPI 离线语音
- 小面板显示「聆听中 / 识别中 / 你说了什么 / 回复」
- 右键光球：打开配置、退出

## 快速开始

1. 运行 `LingYan.exe`
2. 第一次运行会在 `%APPDATA%\LingYan\config.json` 生成配置
3. 右键光球 → 「配置（config.json）」，填写：

```json
{
  "api_key": "你的智谱Key",
  "base_url": "https://open.bigmodel.cn/api/paas/v4",
  "model": "glm-4.5-flash",
  "proxy": "",
  "system_prompt": "你是运行在 Windows 桌面角落的小助手「灵言」。...",
  "tts_rate": 1,
  "position": "corner"
}
```

4. 保存后点光球说话

### 其他大模型

| 服务 | base_url | model |
|---|---|---|
| 智谱 | `https://open.bigmodel.cn/api/paas/v4` | `glm-4.5-flash` |
| DeepSeek | `https://api.deepseek.com` | `deepseek-chat` |
| 硅基流动 | `https://api.siliconflow.cn/v1` | `Qwen/Qwen2.5-7B-Instruct` |
| Groq | `https://api.groq.com/openai/v1` | `llama-3.1-8b-instant` |
| Ollama | `http://127.0.0.1:11434/v1` | `qwen2.5:7b` |

## Windhawk 集成

仓库里的 `windhawk/lingyan-widget.wh.cpp` 是一个 Windhawk mod：

- 目标进程：`explorer.exe`
- 随 explorer 启动时拉起 `LingYan.exe`
- 设置项 `exePath` 指定 LingYan.exe 路径
- 用命名互斥量避免重复启动

安装：

1. 安装 [Windhawk](https://windhawk.net/)
2. 新建 mod，把 `windhawk/lingyan-widget.wh.cpp` 内容粘贴进去
3. 编译并启用，设置 `LingYan.exe` 的完整路径
4. 注销/重启 explorer 后生效

> 任务栏内嵌图标（真正画在任务栏上）需要 hook Windows 11 XAML 任务栏，
> 风险较高，放在 roadmap 里；当前用屏幕角落模式。

## 构建

```powershell
cargo build --release
```

产物：`target\release\LingYan.exe`

依赖：Rust stable + MSVC Build Tools + Windows 10/11。

## 目录

```
LingYan/
├─ src/
│  ├─ main.rs       窗口、状态机、点击/右键菜单
│  ├─ orb.rs        Siri 液态玻璃渲染
│  ├─ recorder.rs   麦克风录音 + 静音检测
│  ├─ sapi_stt.rs   Windows 中文识别
│  ├─ speak.rs      Windows SAPI 朗读
│  ├─ llm.rs        OpenAI 兼容客户端
│  └─ config.rs     config.json
├─ windhawk/
│  └─ lingyan-widget.wh.cpp
├─ config.example.json
└─ README.md
```

## Roadmap

- [ ] Windhawk 任务栏内嵌图标（XAML 任务栏）
- [ ] Vosk 中文流式识别（更低延迟）
- [ ] Edge TTS 晓晓语音
- [ ] 全局热键 / 托盘图标
- [ ] 开机自启

## 隐私

- 录音只在本机处理（SAPI 离线识别）
- 识别文本发送给你配置的大模型服务
- 回复文本交给 Windows SAPI 朗读，不联网
- API Key 明文保存在本机 `config.json`

## 许可证

GPL-3.0-only
