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

#[cfg(test)]
mod tests {
    use super::endpoint;

    #[test]
    fn endpoint_joins_base_url_and_path() {
        assert_eq!(
            endpoint("https://api.example.com/openai/", "/v1/models")
                .unwrap()
                .as_str(),
            "https://api.example.com/openai/v1/models"
        );
    }

    #[test]
    fn endpoint_rejects_full_transcription_endpoint_as_base_url() {
        let error = endpoint(
            "https://api.example.com/v1/audio/transcriptions",
            "/v1/audio/transcriptions",
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Base URL must not include an audio endpoint"));
    }

    #[test]
    fn endpoint_rejects_full_translation_endpoint_as_base_url() {
        let error = endpoint(
            "https://api.example.com/v1/audio/translations",
            "/v1/audio/translations",
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Base URL must not include an audio endpoint"));
    }
}
