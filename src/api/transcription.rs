use crate::api::endpoints::endpoint;
use serde::Deserialize;

pub const AUDIO_RESPONSE_FORMAT: &str = "json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioRequestKind {
    Transcription,
    Translation,
}

#[derive(Debug, Deserialize)]
struct AudioTextResponse {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorEnvelope {
    error: Option<ApiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    message: Option<String>,
    code: Option<String>,
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
    response_body: &str,
) -> String {
    let mut message = format!("Audio API request failed: {status} {reason}. Model: {model}.");
    if let Some(api_message) = safe_api_error_message(response_body) {
        message.push(' ');
        message.push_str(&api_message);
    }
    if kind == AudioRequestKind::Translation {
        message.push_str(" The selected model may not support audio translation.");
    }
    message
}

fn safe_api_error_message(response_body: &str) -> Option<String> {
    let envelope = serde_json::from_str::<ApiErrorEnvelope>(response_body).ok()?;
    let error = envelope.error?;
    let detail = error.message?.trim().replace(['\r', '\n'], " ");
    if detail.is_empty() {
        return None;
    }

    let detail = detail.chars().take(300).collect::<String>();
    match error.code.filter(|code| !code.trim().is_empty()) {
        Some(code) => Some(format!("API error {code}: {detail}")),
        None => Some(format!("API error: {detail}")),
    }
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
        .text("model", audio.model.to_string())
        .text("response_format", AUDIO_RESPONSE_FORMAT);

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
    use super::{
        AUDIO_RESPONSE_FORMAT, AudioRequestKind, AudioTextResponse, audio_endpoint_path,
        safe_api_error_message, sanitize_api_error,
    };

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
    fn audio_response_format_matches_ai2npu_json_contract() {
        assert_eq!(AUDIO_RESPONSE_FORMAT, "json");
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
    fn api_error_body_includes_safe_error_message() {
        let message = sanitize_api_error(
            AudioRequestKind::Transcription,
            "openai/whisper-large-v3-turbo",
            500,
            "Internal Server Error",
            r#"{"error":{"message":"native GenAI bridge failed","code":"internal_error"}}"#,
        );

        assert!(message.contains("internal_error"));
        assert!(message.contains("native GenAI bridge failed"));
    }

    #[test]
    fn safe_api_error_message_rejects_non_error_json() {
        assert_eq!(
            safe_api_error_message(r#"{"text":"recognized private text"}"#),
            None
        );
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
