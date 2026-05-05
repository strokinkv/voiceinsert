using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

public interface IAudioRecorder
{
    event EventHandler<float>? LevelChanged;
    bool IsRecording { get; }
    void Start();
    Task<byte[]> StopAsync();
}

public interface ITranscriptionClient
{
    Task<string> TranscribeAsync(
        byte[] wavBytes,
        AudioRequestKind requestKind,
        CancellationToken cancellationToken);
}

public interface IClipboardInserter
{
    Task InsertAsync(string text, IntPtr targetWindow);
}

public interface IRecordingOverlayService
{
    void ShowRecording();
    void SetLevel(float level);
    void SetStatus(string status);
    void Hide();
}

public interface ISoundService
{
    void PlayStart();
    void PlayStop();
    void PlayError();
}

public interface ILoggingService
{
    void Error(Exception exception, string safeMessage);
    void Information(string message);
}
