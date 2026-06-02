use super::state::log_error_message;
use crate::api::models::load_models;
use crate::api::transcription::{AudioRequestKind, SendAudioRequest, send_audio};
use crate::clipboard::ClipboardInserter;
use std::future::Future;
use std::sync::mpsc::Sender;

#[derive(Debug)]
pub(super) enum BackgroundEvent {
    Error(String),
    ModelsLoaded {
        profile_id: String,
        models: Vec<String>,
    },
    VoiceInsertionStarted,
    VoiceInsertionComplete(Result<(), String>),
}

pub(super) struct AudioTask {
    pub(super) base_url: String,
    pub(super) api_key: String,
    pub(super) model: String,
    pub(super) language: Option<String>,
    pub(super) temperature: f64,
    pub(super) wav_bytes: Vec<u8>,
    pub(super) kind: AudioRequestKind,
    pub(super) target_window: Option<isize>,
}

pub(super) fn spawn_voice_insert_task(
    handle: tokio::runtime::Handle,
    http: reqwest::Client,
    clipboard: ClipboardInserter,
    task: AudioTask,
    events: Sender<BackgroundEvent>,
) {
    spawn_reported(handle, events.clone(), async move {
        let completion = async {
            let AudioTask {
                base_url,
                api_key,
                model,
                language,
                temperature,
                wav_bytes,
                kind,
                target_window,
            } = task;

            let text = send_audio(
                &http,
                SendAudioRequest {
                    base_url: &base_url,
                    api_key: &api_key,
                    model: &model,
                    language: language.as_deref(),
                    temperature: Some(temperature),
                    wav_bytes,
                    kind,
                },
            )
            .await?;

            let _ = events.send(BackgroundEvent::VoiceInsertionStarted);
            clipboard.insert_text(&text, target_window).await
        }
        .await;

        let _ = events.send(BackgroundEvent::VoiceInsertionComplete(
            completion.map_err(voice_completion_error),
        ));
    });
}

pub(super) fn spawn_load_models_task(
    handle: tokio::runtime::Handle,
    http: reqwest::Client,
    profile_id: String,
    base_url: String,
    api_key: String,
    events: Sender<BackgroundEvent>,
) {
    spawn_reported(handle, events.clone(), async move {
        match load_models(&http, &base_url, &api_key).await {
            Ok(models) => {
                let model_ids = models.into_iter().map(|model| model.id).collect();
                let _ = events.send(BackgroundEvent::ModelsLoaded {
                    profile_id,
                    models: model_ids,
                });
            }
            Err(error) => {
                let _ = events.send(BackgroundEvent::Error(log_error_message(&error)));
            }
        }
    });
}

fn spawn_reported<F>(handle: tokio::runtime::Handle, events: Sender<BackgroundEvent>, future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let join = handle.spawn(future);
    handle.spawn(async move {
        if let Err(error) = join.await {
            let _ = events.send(BackgroundEvent::Error(format!(
                "Background task failed: {error}"
            )));
        }
    });
}

fn voice_completion_error(error: anyhow::Error) -> String {
    log_error_message(&error)
}

#[cfg(test)]
mod tests {
    #[test]
    fn voice_completion_error_uses_sanitized_root_cause() {
        let error =
            anyhow::anyhow!("clipboard failed\nprivate recognized text").context("outer context");

        assert_eq!(super::voice_completion_error(error), "clipboard failed");
    }
}
