use reqwest::Url;

pub fn endpoint(base_url: &str, path: &str) -> anyhow::Result<Url> {
    let base = base_url.trim();
    if base.contains("/v1/audio/transcriptions") || base.contains("/v1/audio/translations") {
        anyhow::bail!("Base URL must not include an audio endpoint");
    }

    let mut normalized = base.trim_end_matches('/').to_string();
    normalized.push('/');
    let url = Url::parse(&normalized)?;
    Ok(url.join(path.trim_start_matches('/'))?)
}
