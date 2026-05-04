using System.Runtime.InteropServices;
using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

public sealed class VoiceInsertController(
    SettingsService settings,
    AudioRecorder recorder,
    TranscriptionClient transcriptionClient,
    ClipboardInserter clipboardInserter,
    RecordingOverlayService overlay,
    SoundService sounds,
    LoggingService logs)
{
    private CancellationTokenSource? _operationCts;
    private IntPtr _targetWindow;
    private readonly SemaphoreSlim _operationLock = new(1, 1);
    private DateTimeOffset? _silenceStartedAt;
    private bool _autoStopRequested;
    private bool _stopOnSilenceForCurrentRecording;
    private string _pendingStopReason = "Manual";
    private AudioRequestKind _requestKind = AudioRequestKind.Transcription;

    public bool IsRecording => recorder.IsRecording;

    public async Task HandleHotkeyPressedAsync(AudioRequestKind requestKind = AudioRequestKind.Transcription)
    {
        switch (settings.Current.RecordingMode)
        {
            case RecordingMode.Hold:
                StartRecording(requestKind);
                break;
            case RecordingMode.Toggle:
            case RecordingMode.SilenceTimeout:
                await ToggleAsync(requestKind);
                break;
            default:
                await ToggleAsync(requestKind);
                break;
        }
    }

    public async Task HandleHotkeyReleasedAsync()
    {
        if (settings.Current.RecordingMode == RecordingMode.Hold)
        {
            await StopAndTranscribeAsync();
        }
    }

    private async Task ToggleAsync(AudioRequestKind requestKind)
    {
        if (recorder.IsRecording)
        {
            await StopAndTranscribeAsync();
            return;
        }

        StartRecording(requestKind);
    }

    public void StartRecording(AudioRequestKind requestKind = AudioRequestKind.Transcription)
    {
        if (recorder.IsRecording)
        {
            return;
        }

        try
        {
            _operationCts = new CancellationTokenSource();
            _targetWindow = GetForegroundWindow();
            _silenceStartedAt = null;
            _autoStopRequested = false;
            _stopOnSilenceForCurrentRecording = RecordingModePolicy.StopsOnSilence(settings.Current.RecordingMode);
            _requestKind = requestKind;
            _pendingStopReason = "Manual";
            recorder.LevelChanged += OnLevelChanged;
            recorder.Start();
            overlay.ShowRecording();
            sounds.PlayStart();
            logs.Information($"Recording started. Kind: {_requestKind}. Mode: {settings.Current.RecordingMode}. Silence stop: {_stopOnSilenceForCurrentRecording}.");

            _ = StopAfterMaxDurationAsync(_operationCts.Token);
        }
        catch (Exception exception)
        {
            HandleError(exception, "Failed to start recording.");
        }
    }

    public async Task StopAndTranscribeAsync()
    {
        await StopAndTranscribeAsync("Manual");
    }

    private async Task StopAndTranscribeAsync(string reason)
    {
        if (!await _operationLock.WaitAsync(0))
        {
            return;
        }

        if (!recorder.IsRecording)
        {
            _operationLock.Release();
            return;
        }

        try
        {
            _pendingStopReason = reason;
            overlay.SetStatus(GetOverlayText(_requestKind == AudioRequestKind.Translation
                ? static texts => texts.OverlayTranslating
                : static texts => texts.OverlayTranscribing));
            sounds.PlayStop();
            recorder.LevelChanged -= OnLevelChanged;
            var audio = await recorder.StopAsync();
            logs.Information($"Recording stopped. Reason: {_pendingStopReason}. Audio bytes: {audio.Length}.");

            if (audio.Length == 0)
            {
                throw new InvalidOperationException("Recorded audio is empty.");
            }

            var text = await transcriptionClient.TranscribeAsync(
                audio,
                _requestKind,
                _operationCts?.Token ?? CancellationToken.None);
            if (string.IsNullOrWhiteSpace(text))
            {
                throw new InvalidOperationException("Transcription response text is empty.");
            }

            overlay.SetStatus(GetOverlayText(static texts => texts.OverlayInserting));
            logs.Information("Starting clipboard insertion.");
            await clipboardInserter.InsertAsync(text, _targetWindow);
            logs.Information($"{_requestKind} inserted.");
        }
        catch (Exception exception)
        {
            HandleError(exception, "Voice insertion failed.");
        }
        finally
        {
            _operationCts?.Dispose();
            _operationCts = null;
            overlay.Hide();
            _operationLock.Release();
        }
    }

    private async Task StopAfterMaxDurationAsync(CancellationToken cancellationToken)
    {
        try
        {
            await Task.Delay(TimeSpan.FromSeconds(settings.Current.MaxRecordingSeconds), cancellationToken);
            await StopAndTranscribeAsync("MaxDuration");
        }
        catch (OperationCanceledException)
        {
        }
    }

    private void OnLevelChanged(object? sender, float level)
    {
        overlay.SetLevel(level);

        if (!_stopOnSilenceForCurrentRecording || _autoStopRequested)
        {
            return;
        }

        var threshold = Math.Clamp(settings.Current.SilenceThresholdPercent, 0, 100) / 100f;
        var now = DateTimeOffset.Now;

        if (level <= threshold)
        {
            _silenceStartedAt ??= now;

            if ((now - _silenceStartedAt.Value).TotalMilliseconds >= settings.Current.SilenceTimeoutMilliseconds)
            {
                _autoStopRequested = true;
                _ = StopAndTranscribeAsync("SilenceTimeout");
            }

            return;
        }

        _silenceStartedAt = null;
    }

    private void HandleError(Exception exception, string safeMessage)
    {
        recorder.LevelChanged -= OnLevelChanged;
        overlay.SetStatus(GetOverlayText(static texts => texts.OverlayError));
        sounds.PlayError();
        logs.Error(exception, safeMessage);
    }

    private string GetOverlayText(Func<SettingsTexts, string> selector)
    {
        return selector(SettingsTexts.For(settings.Current.UiLanguage));
    }

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();
}
