const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "VoiceInsert";

#[cfg(windows)]
pub fn is_enabled() -> anyhow::Result<bool> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_READ, REG_VALUE_TYPE, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };
    use windows::core::PCWSTR;

    let mut key = Default::default();
    let run_key = wide_null(RUN_KEY);
    let value_name = wide_null(VALUE_NAME);

    unsafe {
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(run_key.as_ptr()),
            Some(0),
            KEY_READ,
            &mut key,
        );
        if result != ERROR_SUCCESS {
            return Ok(false);
        }

        let mut value_type = REG_VALUE_TYPE::default();
        let result = RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            None,
        );
        let _ = RegCloseKey(key);

        Ok(result == ERROR_SUCCESS)
    }
}

#[cfg(windows)]
pub fn set_enabled(enabled: bool, exe_path: &std::path::Path) -> anyhow::Result<()> {
    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteValueW,
        RegSetValueExW,
    };
    use windows::core::PCWSTR;

    let mut key = Default::default();
    let run_key = wide_null(RUN_KEY);
    let value_name = wide_null(VALUE_NAME);

    unsafe {
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(run_key.as_ptr()),
            Some(0),
            None,
            Default::default(),
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        );
        ensure_success(result, "failed to open autostart registry key")?;

        if enabled {
            let command = format!("\"{}\"", exe_path.display());
            let bytes = wide_null(&command)
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<u8>>();
            let result = RegSetValueExW(
                key,
                PCWSTR(value_name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(&bytes),
            );
            ensure_success(result, "failed to set autostart registry value")?;
        } else {
            let _ = RegDeleteValueW(key, PCWSTR(value_name.as_ptr()));
        }

        let _ = RegCloseKey(key);
    }

    Ok(())
}

#[cfg(windows)]
fn ensure_success(
    result: windows::Win32::Foundation::WIN32_ERROR,
    context: &str,
) -> anyhow::Result<()> {
    use windows::Win32::Foundation::ERROR_SUCCESS;

    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        anyhow::bail!("{context}: {result:?}")
    }
}

#[cfg(not(windows))]
pub fn is_enabled() -> anyhow::Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool, _exe_path: &std::path::Path) -> anyhow::Result<()> {
    Ok(())
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::wide_null;

    #[test]
    fn wide_null_appends_terminator() {
        let value = wide_null("VoiceInsert");

        assert_eq!(value.last(), Some(&0));
    }
}
