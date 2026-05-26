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

pub async fn send_audio(
    http: &reqwest::Client,
    audio: SendAudioRequest<'_>,
) -> anyhow::Result<String> {
    let path = match audio.kind {
        AudioRequestKind::Transcription => "/v1/audio/transcriptions",
        AudioRequestKind::Translation => "/v1/audio/translations",
    };

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

    let parsed = response.json::<AudioTextResponse>().await?;
    parsed
        .text
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("Audio response does not contain a text field."))
}
