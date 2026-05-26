use crate::api::endpoints::endpoint;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioRequestKind {
    Transcription,
    Translation,
}

#[derive(Debug, Deserialize)]
struct AudioTextResponse {
    text: Option<String>,
}

impl AudioTextResponse {
    fn into_text(self) -> anyhow::Result<String> {
        self.text
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("Audio response does not contain a text field."))
    }
}

pub struct SendAudioRequest<'a> {
    pub base_url: &'a str,
    pub api_key: &'a str,
    pub model: &'a str,
    pub language: Option<&'a str>,
    pub temperature: Option<f64>,
    pub wav_bytes: Vec<u8>,
    pub kind: AudioRequestKind,
}

pub fn sanitize_api_error(
    kind: AudioRequestKind,
    model: &str,
    status: u16,
    reason: &str,
    _response_body: &str,
) -> String {
    let mut message = format!("Audio API request failed: {status} {reason}. Model: {model}.");
    if kind == AudioRequestKind::Translation {
        message.push_str(" The selected model may not support audio translation.");
    }
    message
}

pub fn audio_endpoint_path(kind: AudioRequestKind) -> &'static str {
    match kind {
        AudioRequestKind::Transcription => "/v1/audio/transcriptions",
        AudioRequestKind::Translation => "/v1/audio/translations",
    }
}

pub async fn send_audio(
    http: &reqwest::Client,
    audio: SendAudioRequest<'_>,
) -> anyhow::Result<String> {
    let path = audio_endpoint_path(audio.kind);

    let file = reqwest::multipart::Part::bytes(audio.wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", file)
        .text("model", audio.model.to_string());

    if audio.kind == AudioRequestKind::Transcription
        && let Some(language) = audio.language.filter(|value| !value.trim().is_empty())
    {
        form = form.text("language", language.to_string());
    }

    if let Some(temperature) = audio.temperature {
        form = form.text("temperature", temperature.to_string());
    }

    let mut request = http.post(endpoint(audio.base_url, path)?).multipart(form);
    if !audio.api_key.trim().is_empty() {
        request = request.bearer_auth(audio.api_key);
    }

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("HTTP error");
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "{}",
            sanitize_api_error(audio.kind, audio.model, status.as_u16(), reason, &body)
        );
    }

    response.json::<AudioTextResponse>().await?.into_text()
}

#[cfg(test)]
mod tests {
    use super::{AudioRequestKind, AudioTextResponse, audio_endpoint_path, sanitize_api_error};

    #[test]
    fn audio_endpoint_path_matches_request_kind() {
        assert_eq!(
            audio_endpoint_path(AudioRequestKind::Transcription),
            "/v1/audio/transcriptions"
        );
        assert_eq!(
            audio_endpoint_path(AudioRequestKind::Translation),
            "/v1/audio/translations"
        );
    }

    #[test]
    fn translation_error_mentions_model_without_response_body() {
        let message = sanitize_api_error(
            AudioRequestKind::Translation,
            "whisper-large-v3",
            422,
            "Unprocessable Entity",
            r#"{"text":"private recognized user text","api_key":"sk-secret"}"#,
        );

        assert!(message.contains("422"));
        assert!(message.contains("Unprocessable Entity"));
        assert!(message.contains("whisper-large-v3"));
        assert!(message.contains("may not support audio translation"));
        assert!(!message.contains("private recognized user text"));
        assert!(!message.contains("sk-secret"));
    }

    #[test]
    fn transcription_error_does_not_add_translation_hint() {
        let message = sanitize_api_error(
            AudioRequestKind::Transcription,
            "whisper-large-v3",
            500,
            "Internal Server Error",
            "private text",
        );

        assert!(message.contains("500"));
        assert!(message.contains("whisper-large-v3"));
        assert!(!message.contains("may not support audio translation"));
        assert!(!message.contains("private text"));
    }

    #[test]
    fn audio_text_response_rejects_missing_or_empty_text() {
        let missing: AudioTextResponse = serde_json::from_str("{}").unwrap();
        let empty: AudioTextResponse = serde_json::from_str(r#"{ "text": "   " }"#).unwrap();

        assert!(
            missing
                .into_text()
                .unwrap_err()
                .to_string()
                .contains("text field")
        );
        assert!(
            empty
                .into_text()
                .unwrap_err()
                .to_string()
                .contains("text field")
        );
    }

    #[test]
    fn audio_text_response_accepts_text() {
        let response: AudioTextResponse =
            serde_json::from_str(r#"{ "text": "recognized text" }"#).unwrap();

        assert_eq!(response.into_text().unwrap(), "recognized text");
    }
}
