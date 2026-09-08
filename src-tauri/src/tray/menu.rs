use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    AppHandle,
};

/// Build the idle-state tray menu.
pub fn build_idle_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let start = MenuItem::with_id(app, "start_meeting", "开始会议", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let copy_ai = MenuItem::with_id(app, "copy_ai_answer", "复制上一个 AI 回答", true, None::<&str>)?;
    let copy_actions = MenuItem::with_id(app, "copy_action_items", "复制行动项", true, None::<&str>)?;
    let copy_summary = MenuItem::with_id(app, "copy_summary", "复制摘要", true, None::<&str>)?;
    let copy_transcript = MenuItem::with_id(app, "copy_transcript", "复制转录", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 NexQ", true, None::<&str>)?;

    Menu::with_items(app, &[
        &start, &sep1,
        &copy_ai, &copy_actions, &copy_summary, &copy_transcript, &sep2,
        &settings, &quit,
    ])
}

/// Build the meeting-active tray menu.
pub fn build_meeting_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let stop = MenuItem::with_id(app, "stop_meeting", "结束会议", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let mute_mic = MenuItem::with_id(app, "toggle_mic", "麦克风静音", true, None::<&str>)?;
    let mute_sys = MenuItem::with_id(app, "toggle_system", "系统声音静音", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let stealth = MenuItem::with_id(app, "toggle_stealth", "隐身模式", true, None::<&str>)?;
    let show_overlay = MenuItem::with_id(app, "show_overlay", "显示悬浮窗", true, None::<&str>)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let copy_ai = MenuItem::with_id(app, "copy_ai_answer", "复制上一个 AI 回答", true, None::<&str>)?;
    let copy_actions = MenuItem::with_id(app, "copy_action_items", "复制行动项", true, None::<&str>)?;
    let copy_summary = MenuItem::with_id(app, "copy_summary", "复制摘要", true, None::<&str>)?;
    let copy_transcript = MenuItem::with_id(app, "copy_transcript", "复制转录", true, None::<&str>)?;
    let sep4 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 NexQ", true, None::<&str>)?;

    Menu::with_items(app, &[
        &stop, &sep1,
        &mute_mic, &mute_sys, &sep2,
        &stealth, &show_overlay, &sep3,
        &copy_ai, &copy_actions, &copy_summary, &copy_transcript, &sep4,
        &settings, &quit,
    ])
}
