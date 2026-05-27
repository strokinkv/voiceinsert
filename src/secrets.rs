use crate::paths::AppPaths;
use std::collections::BTreeMap;
use std::fs;

pub fn serialize_keys(keys: &BTreeMap<String, String>) -> anyhow::Result<Vec<u8>> {
    Ok(serde_json::to_vec(keys)?)
}

pub fn deserialize_keys(bytes: &[u8]) -> anyhow::Result<BTreeMap<String, String>> {
    let text = String::from_utf8(bytes.to_vec())?;
    if text.trim_start().starts_with('{') {
        Ok(serde_json::from_str(&text)?)
    } else {
        let mut keys = BTreeMap::new();
        keys.insert("default".to_string(), text);
        Ok(keys)
    }
}

pub fn load_api_keys(paths: &AppPaths) -> anyhow::Result<BTreeMap<String, String>> {
    let path = paths.secrets_path();
    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    let protected = fs::read(path)?;
    let serialized = unprotect(&protected)?;
    deserialize_keys(&serialized)
}

pub fn save_api_keys(paths: &AppPaths, keys: &BTreeMap<String, String>) -> anyhow::Result<()> {
    fs::create_dir_all(paths.app_data_dir())?;
    if keys.is_empty() {
        let path = paths.secrets_path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    let serialized = serialize_keys(keys)?;
    let protected = protect(&serialized)?;
    fs::write(paths.secrets_path(), protected)?;
    Ok(())
}

#[cfg(windows)]
pub fn protect(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptProtectData};
    use windows::core::PCWSTR;

    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut output)?;
    }

    let protected = unsafe {
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let protected = slice.to_vec();
        LocalFree(Some(HLOCAL(output.pbData.cast())));
        protected
    };

    Ok(protected)
}

#[cfg(windows)]
pub fn unprotect(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};

    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptUnprotectData(&input, None, None, None, None, 0, &mut output)?;
    }

    let unprotected = unsafe {
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let unprotected = slice.to_vec();
        LocalFree(Some(HLOCAL(output.pbData.cast())));
        unprotected
    };

    Ok(unprotected)
}

#[cfg(not(windows))]
pub fn protect(_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    anyhow::bail!("DPAPI is only available on Windows")
}

#[cfg(not(windows))]
pub fn unprotect(_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    anyhow::bail!("DPAPI is only available on Windows")
}
