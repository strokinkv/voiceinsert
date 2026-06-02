/// Desktop work area available for positioning transient overlay windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// Returns the primary monitor work area, excluding the taskbar when the platform reports it.
pub fn primary_work_area() -> WorkArea {
    platform_work_area().unwrap_or_else(default_work_area)
}

/// Calculates a bottom-right overlay position clamped inside the supplied work area.
pub fn overlay_position(work_area: WorkArea, overlay_size: (i32, i32), margin: i32) -> (i32, i32) {
    let (overlay_width, overlay_height) = overlay_size;
    let margin = margin.max(0);
    let x = (work_area.left + work_area.width - overlay_width - margin).max(work_area.left);
    let y = (work_area.top + work_area.height - overlay_height - margin).max(work_area.top);
    (x, y)
}

#[cfg(windows)]
fn platform_work_area() -> Option<WorkArea> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };

    let mut rect = RECT::default();
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut rect as *mut RECT).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };

    ok.is_ok().then_some(WorkArea {
        left: rect.left,
        top: rect.top,
        width: rect.right.saturating_sub(rect.left),
        height: rect.bottom.saturating_sub(rect.top),
    })
}

#[cfg(not(windows))]
fn platform_work_area() -> Option<WorkArea> {
    None
}

#[cfg(windows)]
fn default_work_area() -> WorkArea {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    unsafe {
        WorkArea {
            left: 0,
            top: 0,
            width: GetSystemMetrics(SM_CXSCREEN),
            height: GetSystemMetrics(SM_CYSCREEN),
        }
    }
}

#[cfg(not(windows))]
fn default_work_area() -> WorkArea {
    WorkArea {
        left: 0,
        top: 0,
        width: 1280,
        height: 720,
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkArea, overlay_position};

    #[test]
    fn overlay_position_stays_within_work_area() {
        let work_area = WorkArea {
            left: 100,
            top: 50,
            width: 800,
            height: 600,
        };

        assert_eq!(overlay_position(work_area, (240, 120), 24), (636, 506));
        assert_eq!(overlay_position(work_area, (900, 700), 24), (100, 50));
    }
}
