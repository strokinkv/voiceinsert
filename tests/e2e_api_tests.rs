use voiceinsert::api::transcription::{SendAudioRequest, send_audio};
use voiceinsert::audio::recorder::encode_wav_mono_16khz_i16;
use wiremock::matchers::{method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

struct MultipartField {
    name: &'static str,
    value: &'static str,
}

impl Match for MultipartField {
    fn matches(&self, request: &Request) -> bool {
        multipart_field_value(&request.body, self.name)
            .is_some_and(|value| value.trim() == self.value)
    }
}

fn multipart_field_value(body: &[u8], name: &str) -> Option<String> {
    let body = String::from_utf8_lossy(body);
    let name_marker = format!("name=\"{name}\"");
    let name_start = body.find(&name_marker)?;
    let value_start = body[name_start..].find("\r\n\r\n")? + name_start + 4;
    let value_end = body[value_start..].find("\r\n--")? + value_start;
    Some(body[value_start..value_end].to_string())
}

#[tokio::test]
async fn transcription_defaults_language_to_russian() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .and(MultipartField {
            name: "language",
            value: "ru",
        })
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
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "hello from mock");
}

#[tokio::test]
async fn explicit_language_overrides_default_language() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .and(MultipartField {
            name: "language",
            value: "en",
        })
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
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "english text");
}
