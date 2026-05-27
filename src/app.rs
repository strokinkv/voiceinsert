#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};

pub fn run() -> anyhow::Result<()> {
    let _instance = SingleInstanceGuard::acquire("VoiceInsertAppMutex")?;
    if !_instance.acquired {
        return Ok(());
    }

    Ok(())
}

#[derive(Debug)]
pub struct SingleInstanceGuard {
    acquired: bool,
    #[cfg(windows)]
    handle: HANDLE,
}

impl SingleInstanceGuard {
    pub fn acquire(name: &str) -> anyhow::Result<Self> {
        acquire_single_instance(name)
    }

    pub fn acquired(&self) -> bool {
        self.acquired
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
fn acquire_single_instance(name: &str) -> anyhow::Result<SingleInstanceGuard> {
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide_name.as_ptr()))? };
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;

    Ok(SingleInstanceGuard {
        acquired: !already_exists,
        handle,
    })
}

#[cfg(not(windows))]
fn acquire_single_instance(_name: &str) -> anyhow::Result<SingleInstanceGuard> {
    Ok(SingleInstanceGuard { acquired: true })
}

#[cfg(test)]
mod tests {
    use super::SingleInstanceGuard;

    #[test]
    fn single_instance_guard_reports_acquired_on_non_conflicting_name() {
        let name = format!("VoiceInsertTestMutex{}", std::process::id());
        let guard = SingleInstanceGuard::acquire(&name).unwrap();

        assert!(guard.acquired());
    }
}
