use tauri::{command, AppHandle, Manager};

const DEFAULT_OVERLAY_OPACITY: f64 = 0.65;

/// Apply the user's opacity setting to the overlay's top-level native window.
///
/// CSS alpha only changes the webview contents. On Windows the overlay is a
/// layered transparent window, so the native alpha is also required for the
/// desktop behind the whole window to remain visible.
#[command]
pub fn set_overlay_opacity(app: AppHandle, opacity: f64) -> Result<(), String> {
    let opacity = normalize_opacity(opacity);
    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| "Overlay window is not available".to_string())?;

    #[cfg(target_os = "windows")]
    {
        let hwnd = window
            .hwnd()
            .map_err(|error| format!("Failed to access overlay window handle: {error}"))?;
        apply_native_window_opacity(hwnd.0, opacity)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        let _ = opacity;
        Ok(())
    }
}

/// Keep the native alpha in the same safe range as the frontend slider.
pub fn normalize_opacity(opacity: f64) -> f64 {
    if opacity.is_finite() {
        opacity.clamp(0.1, 1.0)
    } else {
        DEFAULT_OVERLAY_OPACITY
    }
}

#[cfg(target_os = "windows")]
fn apply_native_window_opacity(
    hwnd_raw: *mut std::ffi::c_void,
    opacity: f64,
) -> Result<(), String> {
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
        LWA_ALPHA, WS_EX_LAYERED,
    };

    let hwnd = HWND(hwnd_raw);
    let alpha = (normalize_opacity(opacity) * 255.0).round().clamp(1.0, 255.0) as u8;

    // Tauri normally adds WS_EX_LAYERED for transparent windows. Ensure it is
    // present before calling SetLayeredWindowAttributes so packaged Windows
    // builds and development windows follow the same path.
    unsafe {
        let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let layered_style = current_style | WS_EX_LAYERED.0 as isize;
        if layered_style != current_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, layered_style);
        }

        SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)
            .map_err(|error| format!("Failed to apply overlay opacity: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_opacity;

    #[test]
    fn clamps_native_opacity_to_the_supported_range() {
        assert_eq!(normalize_opacity(-1.0), 0.1);
        assert_eq!(normalize_opacity(0.65), 0.65);
        assert_eq!(normalize_opacity(2.0), 1.0);
        assert_eq!(normalize_opacity(f64::NAN), 0.65);
    }
}
