#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod llm;
mod orb;
mod recorder;
mod sapi_stt;
mod speak;

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, Sender};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, AlphaBlend, BeginPaint, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
    BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush,
    DIB_RGB_COLORS, DeleteObject, EndPaint, FillRect, GetDC, InvalidateRect, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, SetWindowRgn, UpdateWindow, CLIP_DEFAULT_PRECIS,
    CLEARTYPE_QUALITY, DEFAULT_CHARSET, DEFAULT_PITCH, FW_NORMAL, OUT_DEFAULT_PRECIS,
    PAINTSTRUCT, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, GetWindowLongPtrW, LoadCursorW, MessageBoxW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetProcessDPIAware, SetForegroundWindow, SetLayeredWindowAttributes, SetTimer,
    SetWindowTextW, ShowWindow, SystemParametersInfoW, TrackPopupMenu, TranslateMessage,
    SetWindowPos, UpdateLayeredWindow, ULW_ALPHA, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, IDC_ARROW,
    LWA_ALPHA, LWA_COLORKEY, MB_ICONINFORMATION, MF_STRING, MSG, SPI_GETWORKAREA, SW_HIDE, SW_SHOWNOACTIVATE,
    TPM_NONOTIFY, TPM_RETURNCMD, WM_CTLCOLORSTATIC, WM_DESTROY, WM_LBUTTONUP, WM_PAINT,
    WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_CHILD, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

const ORB_SIZE: i32 = 132;
const PANEL_W: i32 = 430;
const PANEL_H: i32 = 150;
const TIMER_ID: usize = 1;

static APP: OnceLock<Mutex<App>> = OnceLock::new();

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    Listening,
    Thinking,
    Speaking,
    Reply,
    Error,
}

enum Event {
    Status(String),
    Reply(String),
    Done,
    Error(String),
}

struct App {
    orb: usize,
    panel: usize,
    label: usize,
    mem_dc: usize,
    dib: usize,
    bits: usize,
    pixels: Vec<u32>,
    brush: usize,
    orb_x: i32,
    orb_y: i32,
    state: State,
    config: config::Config,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    hide_at: Option<Instant>,
    started: Instant,
}

unsafe impl Send for App {}

fn app() -> &'static Mutex<App> {
    APP.get().expect("app state")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

fn main() {
    let config = config::Config::load_or_create();
    let (tx, rx) = unbounded();
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };

    let brush = unsafe { CreateSolidBrush(rgb(18, 18, 26)) } as usize;
    let key_brush = unsafe { CreateSolidBrush(rgb(255, 0, 255)) };

    let orb_class = wide("LingYanOrb");
    let panel_class = wide("LingYanPanel");
    unsafe {
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(orb_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: key_brush as *mut c_void,
            lpszMenuName: std::ptr::null(),
            lpszClassName: orb_class.as_ptr(),
        };
        RegisterClassW(&wc);
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(panel_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: brush as *mut c_void,
            lpszMenuName: std::ptr::null(),
            lpszClassName: panel_class.as_ptr(),
        };
        RegisterClassW(&wc);
    }

    let mut work: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut work as *mut _ as *mut c_void,
            0,
        );
    }
    let screen_w = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows_sys::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
        )
    };
    let screen_h = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows_sys::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
        )
    };
    let orb_x = 24;
    let orb_y = 24;
    let panel_x = 24;
    let panel_y = orb_y + ORB_SIZE + 10;

    let title = wide("灵言");
    let panel = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            panel_class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            panel_x,
            panel_y,
            PANEL_W,
            PANEL_H,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    unsafe {
        SetLayeredWindowAttributes(panel, 0, 238, LWA_ALPHA);
    }

    let font = unsafe {
        CreateFontW(
            -17,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide("Microsoft YaHei UI").as_ptr(),
        )
    };
    let label_text = wide("聆听中…请说话");
    let label = unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            label_text.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            14,
            12,
            PANEL_W - 28,
            PANEL_H - 24,
            panel,
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        SendMessageW(hwnd, 0x0030, font as usize, 1);
        hwnd
    };

    let orb = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            orb_class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            orb_x,
            orb_y,
            ORB_SIZE,
            ORB_SIZE,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };

    let mem_dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: ORB_SIZE,
            biHeight: -ORB_SIZE,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [unsafe { std::mem::zeroed() }],
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let dib = unsafe {
        CreateDIBSection(
            std::ptr::null_mut(),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        )
    };
    unsafe {
        SelectObject(mem_dc, dib);
        ShowWindow(orb, SW_SHOWNOACTIVATE);
        SetWindowPos(
            orb,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        SetTimer(orb, TIMER_ID, 16, None);
    }

    let pixels = vec![0u32; (ORB_SIZE * ORB_SIZE) as usize];
    let app_state = App {
        orb: orb as usize,
        panel: panel as usize,
        label: label as usize,
        mem_dc: mem_dc as usize,
        dib: dib as usize,
        bits: bits as usize,
        pixels,
        brush,
        orb_x,
        orb_y,
        state: State::Idle,
        config,
        tx,
        rx,
        hide_at: None,
        started: Instant::now(),
    };
    let _ = APP.set(Mutex::new(app_state));

    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
        if result <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn orb_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TIMER => {
            tick();
            0
        }
        WM_PAINT => {
            paint_orb(hwnd);
            0
        }
        WM_LBUTTONUP => {
            on_click();
            0
        }
        WM_RBUTTONUP => {
            show_menu(hwnd);
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe extern "system" fn panel_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CTLCOLORSTATIC => {
            unsafe {
                SetTextColor(lparam as *mut c_void, rgb(240, 242, 250));
                SetBkMode(lparam as *mut c_void, TRANSPARENT as i32);
            }
            app().lock().unwrap().brush as LRESULT
        }
        WM_LBUTTONUP => {
            let mut state = app().lock().unwrap();
            if state.state == State::Reply || state.state == State::Error {
                unsafe {
                    ShowWindow(state.panel as HWND, SW_HIDE);
                }
                state.state = State::Idle;
                state.hide_at = None;
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn show_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, 1, wide("配置（config.json）").as_ptr());
        AppendMenuW(menu, MF_STRING, 2, wide("退出灵言").as_ptr());
        let mut point: POINT = std::mem::zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);
        match command {
            1 => {
                let state = app().lock().unwrap();
                state.config.open_in_notepad();
            }
            2 => {
                unsafe { PostQuitMessage(0) };
            }
            _ => {}
        }
    }
}

fn on_click() {
    let mut state = app().lock().unwrap();
    if state.state == State::Listening
        || state.state == State::Thinking
        || state.state == State::Speaking
    {
        return;
    }
    if state.state == State::Reply || state.state == State::Error {
        unsafe {
            ShowWindow(state.panel as HWND, SW_HIDE);
        }
        state.state = State::Idle;
        state.hide_at = None;
        return;
    }

    state.config = config::Config::load_or_create();
    if state.config.api_key.trim().is_empty() {
        unsafe {
            MessageBoxW(
                state.orb as HWND,
                wide("请先在 config.json 里填写 api_key，然后保存并重试。").as_ptr(),
                wide("灵言").as_ptr(),
                MB_ICONINFORMATION,
            );
        }
        state.config.open_in_notepad();
        return;
    }

    state.state = State::Listening;
    state.hide_at = None;
    set_panel(&mut state, "聆听中…请说话");
    let cfg = state.config.clone();
    let tx = state.tx.clone();
    std::thread::spawn(move || {
        let pcm = match recorder::record_until_silence() {
            Ok(pcm) => pcm,
            Err(error) => {
                let _ = tx.send(Event::Error(error));
                return;
            }
        };
        let _ = tx.send(Event::Status("识别中…".to_owned()));
        let text = match sapi_stt::recognize(&pcm) {
            Ok(text) => text,
            Err(error) => {
                let _ = tx.send(Event::Error(error));
                return;
            }
        };
        if text.trim().is_empty() {
            let _ = tx.send(Event::Error("没有听清，请再说一次".to_owned()));
            return;
        }
        let _ = tx.send(Event::Status(format!("你说：{text}")));
        match llm::chat(&cfg, &text) {
            Ok(reply) => {
                let _ = tx.send(Event::Reply(reply.clone()));
                let _ = tx.send(Event::Status("朗读中…".to_owned()));
                speak::speak(&reply, cfg.tts_rate);
                let _ = tx.send(Event::Done);
            }
            Err(error) => {
                let _ = tx.send(Event::Error(error));
            }
        }
    });
}

fn tick() {
    let mut state = app().lock().unwrap();

    while let Ok(event) = state.rx.try_recv() {
        match event {
            Event::Status(text) => {
                if text.starts_with("识别") {
                    state.state = State::Thinking;
                } else if text.starts_with("朗读") {
                    state.state = State::Speaking;
                }
                set_panel(&mut state, &text);
            }
            Event::Reply(reply) => {
                state.state = State::Speaking;
                set_panel(&mut state, &reply);
            }
            Event::Done => {
                state.state = State::Reply;
                state.hide_at = Some(Instant::now() + Duration::from_secs(12));
            }
            Event::Error(error) => {
                state.state = State::Error;
                state.hide_at = Some(Instant::now() + Duration::from_secs(10));
                set_panel(&mut state, &error);
            }
        }
    }

    if state.hide_at.is_some_and(|at| Instant::now() >= at) {
        state.hide_at = None;
        state.state = State::Idle;
        unsafe {
            ShowWindow(state.panel as HWND, SW_HIDE);
        }
    }

    if state.state == State::Idle {
        unsafe {
            ShowWindow(state.panel as HWND, SW_HIDE);
        }
    }

    let active = matches!(
        state.state,
        State::Listening | State::Thinking | State::Speaking
    );
    let elapsed = state.started.elapsed().as_secs_f32();
    orb::render(
        &mut state.pixels,
        ORB_SIZE as usize,
        ORB_SIZE as usize,
        elapsed,
        active,
    );
    unsafe {
        std::ptr::copy_nonoverlapping(
            state.pixels.as_ptr() as *const u8,
            state.bits as *mut u8,
            state.pixels.len() * 4,
        );
        let screen = GetDC(std::ptr::null_mut());
        let dst = POINT {
            x: state.orb_x,
            y: state.orb_y,
        };
        let size = SIZE {
            cx: ORB_SIZE,
            cy: ORB_SIZE,
        };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        UpdateLayeredWindow(
            state.orb as HWND,
            screen,
            &dst,
            &size,
            state.mem_dc as *mut c_void,
            &src,
            0,
            &blend,
            ULW_ALPHA,
        );
        ReleaseDC(std::ptr::null_mut(), screen);
    }
}

fn set_panel(state: &mut App, text: &str) {
    let text = wide(text);
    unsafe {
        SetWindowTextW(state.label as HWND, text.as_ptr());
        if state.state != State::Idle {
            ShowWindow(state.panel as HWND, SW_SHOWNOACTIVATE);
            UpdateWindow(state.panel as HWND);
        }
    }
}


fn paint_orb(hwnd: HWND) {
    let mut paint: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    unsafe {
        let hdc = BeginPaint(hwnd, &mut paint);
        let key = CreateSolidBrush(rgb(255, 0, 255));
        let rect = RECT {
            left: 0,
            top: 0,
            right: ORB_SIZE,
            bottom: ORB_SIZE,
        };
        FillRect(hdc, &rect, key);
        DeleteObject(key);
        let state = app().lock().unwrap();
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        AlphaBlend(
            hdc,
            0,
            0,
            ORB_SIZE,
            ORB_SIZE,
            state.mem_dc as *mut c_void,
            0,
            0,
            ORB_SIZE,
            ORB_SIZE,
            blend,
        );
        drop(state);
        EndPaint(hwnd, &paint);
    }
}
