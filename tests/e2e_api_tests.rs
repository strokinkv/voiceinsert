use voiceinsert::api::transcription::{AudioRequestKind, SendAudioRequest, send_audio};
use voiceinsert::audio::recorder::encode_wav_mono_16khz_i16;
use wiremock::matchers::{method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

struct BodyContains(&'static [u8]);

impl Match for BodyContains {
    fn matches(&self, request: &Request) -> bool {
        request
            .body
            .windows(self.0.len())
            .any(|window| window == self.0)
    }
}

#[tokio::test]
async fn transcription_defaults_language_to_russian() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .and(BodyContains(b"language"))
        .and(BodyContains(b"ru"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "hello from mock"
        })))
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "openai/whisper-large-v3-turbo",
            language: None,
            temperature: Some(0.2),
            wav_bytes: encode_wav_mono_16khz_i16(&[0, 100, -100]).unwrap(),
            kind: AudioRequestKind::Transcription,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "hello from mock");
}

#[tokio::test]
async fn translation_defaults_input_language_to_russian() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/translations"))
        .and(BodyContains(b"language"))
        .and(BodyContains(b"ru"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "translated text"
        })))
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "whisper-large-v3",
            language: None,
            temperature: Some(0.2),
            wav_bytes: encode_wav_mono_16khz_i16(&[0, 100, -100]).unwrap(),
            kind: AudioRequestKind::Translation,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "translated text");
}

#[tokio::test]
async fn explicit_language_overrides_default_language() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .and(BodyContains(b"language"))
        .and(BodyContains(b"en"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "english text"
        })))
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "openai/whisper-large-v3-turbo",
            language: Some("en"),
            temperature: Some(0.2),
            wav_bytes: encode_wav_mono_16khz_i16(&[0, 100, -100]).unwrap(),
            kind: AudioRequestKind::Transcription,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "english text");
}
