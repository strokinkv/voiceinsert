use voiceinsert::api::endpoints::endpoint;
use voiceinsert::api::models::load_models;
use voiceinsert::api::transcription::{
    AudioRequestKind, SendAudioRequest, sanitize_api_error, send_audio,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

#[tokio::test]
async fn load_models_uses_models_path_and_bearer_authorization() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/v1/models"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                { "id": "whisper-large-v3" },
                { "id": "gpt-4o-mini-transcribe" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let models = load_models(
        &reqwest::Client::new(),
        &format!("{}/openai/", server.uri()),
        "test-key",
    )
    .await
    .unwrap();

    assert_eq!(models[0].id, "whisper-large-v3");
    assert_eq!(models[1].id, "gpt-4o-mini-transcribe");
}

#[tokio::test]
async fn send_audio_posts_transcription_multipart_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "recognized text"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "test-key",
            model: "whisper-large-v3",
            language: Some("ru"),
            temperature: Some(0.2),
            wav_bytes: vec![82, 73, 70, 70],
            kind: AudioRequestKind::Transcription,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "recognized text");
}

#[tokio::test]
async fn send_audio_posts_translation_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/translations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "translated text"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "whisper-large-v3",
            language: Some("ru"),
            temperature: Some(0.1),
            wav_bytes: vec![82, 73, 70, 70],
            kind: AudioRequestKind::Translation,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "translated text");
}

#[tokio::test]
async fn send_audio_translation_error_does_not_leak_response_body_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/translations"))
        .respond_with(ResponseTemplate::new(422).set_body_string(
            r#"{"error":{"message":"private recognized user text sk-secret raw audio bytes"}}"#,
        ))
        .mount(&server)
        .await;

    let error = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "test-key",
            model: "whisper-large-v3",
            language: None,
            temperature: None,
            wav_bytes: vec![82, 73, 70, 70],
            kind: AudioRequestKind::Translation,
        },
    )
    .await
    .unwrap_err()
    .to_string();

    assert!(error.contains("422"));
    assert!(error.contains("whisper-large-v3"));
    assert!(error.contains("may not support audio translation"));
    assert!(!error.contains("private recognized user text"));
    assert!(!error.contains("sk-secret"));
    assert!(!error.contains("raw audio bytes"));
}

#[tokio::test]
async fn send_audio_rejects_missing_or_empty_text_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "   "
        })))
        .mount(&server)
        .await;

    let error = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "whisper-large-v3",
            language: None,
            temperature: None,
            wav_bytes: vec![82, 73, 70, 70],
            kind: AudioRequestKind::Transcription,
        },
    )
    .await
    .unwrap_err()
    .to_string();

    assert!(error.contains("text field"));
}
