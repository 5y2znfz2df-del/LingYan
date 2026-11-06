// ==WindhawkMod==
// @id              lingyan-widget
// @name            灵言 Siri 小部件
// @description     随 explorer 启动灵言语音小部件（屏幕角落 Siri 图标）
// @version         0.1.0
// @author          LingYan
// @github          https://github.com/5y2znfz2df-del/LingYan
// @include         explorer.exe
// @compilerOptions -lshell32
// ==/WindhawkMod==

// ==WindhawkModSettings==
// - exePath: ""
//   $name: LingYan.exe 路径
//   $description: 例如 C:\Program Files\LingYan\LingYan.exe
// - delayMs: 3000
//   $name: 启动延迟（毫秒）
//   $description: 等桌面加载完成后再启动，避免开机时窗口找不到工作区
// ==/WindhawkModSettings==

#include <windows.h>
#include <shellapi.h>
#include <string>

static HANDLE g_mutex = nullptr;

static void LaunchLingYan() {
    PCWSTR configured = Wh_GetStringSetting(L"exePath");
    std::wstring path = configured ? configured : L"";
    if (configured) {
        Wh_FreeStringSetting(configured);
    }
    if (path.empty()) {
        Wh_Log(L"exePath is empty, skip");
        return;
    }

    int delay = Wh_GetIntSetting(L"delayMs");
    if (delay > 0) {
        Sleep((DWORD)delay);
    }

    HINSTANCE result = ShellExecuteW(
        nullptr,
        L"open",
        path.c_str(),
        nullptr,
        nullptr,
        SW_SHOWNORMAL);
    Wh_Log(L"Launch %s -> %p", path.c_str(), result);
}

BOOL Wh_ModInit() {
    // Only one widget instance even if explorer restarts.
    g_mutex = CreateMutexW(nullptr, TRUE, L"Local\\LingYanWidgetSingleton");
    if (g_mutex && GetLastError() == ERROR_ALREADY_EXISTS) {
        CloseHandle(g_mutex);
        g_mutex = nullptr;
        Wh_Log(L"LingYan already running");
        return TRUE;
    }

    LaunchLingYan();
    return TRUE;
}

void Wh_ModUninit() {
    if (g_mutex) {
        ReleaseMutex(g_mutex);
        CloseHandle(g_mutex);
        g_mutex = nullptr;
    }
}
