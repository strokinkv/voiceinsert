using VoiceInsert.App.Models;
using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class VoiceInsertControllerTests
{
    [Fact]
    public async Task StartRecording_IgnoredWhileStopAndTranscribeIsRunning()
    {
        using var fixture = new SettingsFixture(new AppSettings());
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient { WaitForCompletion = true };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        controller.StartRecording();
        var stopTask = controller.StopAndTranscribeAsync();
        await transcription.WaitUntilCalledAsync(TestContext.Current.CancellationToken);

        controller.StartRecording();

        Assert.Equal(1, recorder.StartCalls);
        Assert.True(recorder.StopCalls >= 1);
        transcription.Complete("recognized text");
        await stopTask;
        Assert.Equal(1, clipboard.InsertCalls);
    }

    [Fact]
    public async Task OldMaxDurationTimer_DoesNotStopNewRecording()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            MaxRecordingSeconds = 1
        });
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient { TextToReturn = "recognized text" };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        controller.StartRecording();
        await controller.StopAndTranscribeAsync();
        await Task.Delay(800, TestContext.Current.CancellationToken);

        controller.StartRecording();
        await Task.Delay(350, TestContext.Current.CancellationToken);

        Assert.True(recorder.IsRecording);
        Assert.Equal(1, recorder.StopCalls);

        await controller.StopAndTranscribeAsync();
    }

    [Fact]
    public async Task StopAndTranscribeAsync_DoesNotCallApiWhenAudioIsEmpty()
    {
        using var fixture = new SettingsFixture(new AppSettings());
        var recorder = new FakeAudioRecorder { AudioToReturn = [] };
        var transcription = new FakeTranscriptionClient();
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        controller.StartRecording();
        await controller.StopAndTranscribeAsync();

        Assert.Equal(0, transcription.CallCount);
        Assert.Equal(0, clipboard.InsertCalls);
    }

    [Fact]
    public async Task HandleHotkeyPressedAsync_ToggleMode_StartsThenStops()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            RecordingMode = RecordingMode.Toggle
        });
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient { TextToReturn = "recognized text" };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        await controller.HandleHotkeyPressedAsync();
        Assert.True(recorder.IsRecording);

        await controller.HandleHotkeyPressedAsync();

        Assert.False(recorder.IsRecording);
        Assert.Equal(1, clipboard.InsertCalls);
    }

    [Fact]
    public async Task HandleHotkeyReleasedAsync_HoldMode_StopsRecording()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            RecordingMode = RecordingMode.Hold
        });
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient { TextToReturn = "recognized text" };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        await controller.HandleHotkeyPressedAsync();
        Assert.True(recorder.IsRecording);

        await controller.HandleHotkeyReleasedAsync();

        Assert.False(recorder.IsRecording);
        Assert.Equal(1, clipboard.InsertCalls);
    }

    [Fact]
    public async Task SilenceTimeoutMode_StopsAfterSustainedLowLevel()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            RecordingMode = RecordingMode.SilenceTimeout,
            SilenceThresholdPercent = 10,
            SilenceTimeoutMilliseconds = 100
        });
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient { TextToReturn = "recognized text" };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        await controller.HandleHotkeyPressedAsync();
        recorder.RaiseLevel(0.01f);
        await Task.Delay(130, TestContext.Current.CancellationToken);
        recorder.RaiseLevel(0.01f);
        await clipboard.WaitForInsertAsync(TestContext.Current.CancellationToken);

        Assert.False(recorder.IsRecording);
        Assert.Equal(1, clipboard.InsertCalls);
    }

    [Fact]
    public async Task StopAndTranscribeAsync_DoesNotInsertEmptyApiText()
    {
        using var fixture = new SettingsFixture(new AppSettings());
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient { TextToReturn = "   " };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        controller.StartRecording();
        await controller.StopAndTranscribeAsync();

        Assert.Equal(1, transcription.CallCount);
        Assert.Equal(0, clipboard.InsertCalls);
    }

    [Fact]
    public async Task StopAndTranscribeAsync_DoesNotInsertWhenApiFails()
    {
        using var fixture = new SettingsFixture(new AppSettings());
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient
        {
            ExceptionToThrow = new InvalidOperationException("API failed")
        };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        controller.StartRecording();
        await controller.StopAndTranscribeAsync();

        Assert.Equal(1, transcription.CallCount);
        Assert.Equal(0, clipboard.InsertCalls);
    }

    [Fact]
    public async Task StopAndTranscribeAsync_InsertsReturnedText()
    {
        using var fixture = new SettingsFixture(new AppSettings());
        var recorder = new FakeAudioRecorder { AudioToReturn = [1, 2, 3, 4] };
        var transcription = new FakeTranscriptionClient { TextToReturn = "recognized text" };
        var clipboard = new FakeClipboardInserter();
        var controller = CreateController(fixture.Settings, recorder, transcription, clipboard);

        controller.StartRecording(AudioRequestKind.Translation);
        await controller.StopAndTranscribeAsync();

        Assert.Equal(1, transcription.CallCount);
        Assert.Equal(AudioRequestKind.Translation, transcription.LastRequestKind);
        Assert.Equal(1, clipboard.InsertCalls);
        Assert.Equal("recognized text", clipboard.LastText);
    }

    private static VoiceInsertController CreateController(
        SettingsService settings,
        FakeAudioRecorder recorder,
        FakeTranscriptionClient transcription,
        FakeClipboardInserter clipboard)
    {
        return new VoiceInsertController(
            settings,
            recorder,
            transcription,
            clipboard,
            new FakeRecordingOverlayService(),
            new FakeSoundService(),
            new FakeLoggingService());
    }

    private sealed class FakeAudioRecorder : IAudioRecorder
    {
        public event EventHandler<float>? LevelChanged;
        public bool IsRecording { get; private set; }
        public byte[] AudioToReturn { get; init; } = [1, 2, 3, 4];
        public int StartCalls { get; private set; }
        public int StopCalls { get; private set; }

        public void Start()
        {
            StartCalls++;
            IsRecording = true;
            LevelChanged?.Invoke(this, 0.5f);
        }

        public void RaiseLevel(float level)
        {
            LevelChanged?.Invoke(this, level);
        }

        public Task<byte[]> StopAsync()
        {
            StopCalls++;
            IsRecording = false;
            return Task.FromResult(AudioToReturn);
        }
    }

    private sealed class FakeTranscriptionClient : ITranscriptionClient
    {
        private readonly TaskCompletionSource _called = new(TaskCreationOptions.RunContinuationsAsynchronously);
        private readonly TaskCompletionSource<string> _completion = new(TaskCreationOptions.RunContinuationsAsynchronously);

        public bool WaitForCompletion { get; init; }
        public string TextToReturn { get; init; } = "ok";
        public Exception? ExceptionToThrow { get; init; }
        public int CallCount { get; private set; }
        public AudioRequestKind LastRequestKind { get; private set; }

        public Task WaitUntilCalledAsync(CancellationToken cancellationToken)
        {
            return _called.Task.WaitAsync(cancellationToken);
        }

        public void Complete(string text)
        {
            _completion.TrySetResult(text);
        }

        public async Task<string> TranscribeAsync(
            byte[] wavBytes,
            AudioRequestKind requestKind,
            CancellationToken cancellationToken)
        {
            CallCount++;
            LastRequestKind = requestKind;
            _called.TrySetResult();

            if (ExceptionToThrow is not null)
            {
                throw ExceptionToThrow;
            }

            if (WaitForCompletion)
            {
                return await _completion.Task.WaitAsync(cancellationToken);
            }

            return TextToReturn;
        }
    }

    private sealed class FakeClipboardInserter : IClipboardInserter
    {
        private readonly TaskCompletionSource _inserted = new(TaskCreationOptions.RunContinuationsAsynchronously);

        public int InsertCalls { get; private set; }
        public string LastText { get; private set; } = "";

        public Task WaitForInsertAsync(CancellationToken cancellationToken)
        {
            return _inserted.Task.WaitAsync(cancellationToken);
        }

        public Task InsertAsync(string text, IntPtr targetWindow)
        {
            InsertCalls++;
            LastText = text;
            _inserted.TrySetResult();
            return Task.CompletedTask;
        }
    }

    private sealed class FakeRecordingOverlayService : IRecordingOverlayService
    {
        public void ShowRecording() { }
        public void SetLevel(float level) { }
        public void SetStatus(string status) { }
        public void Hide() { }
    }

    private sealed class FakeSoundService : ISoundService
    {
        public void PlayStart() { }
        public void PlayStop() { }
        public void PlayError() { }
    }

    private sealed class FakeLoggingService : ILoggingService
    {
        public void Error(Exception exception, string safeMessage) { }
        public void Information(string message) { }
    }

    private sealed class SettingsFixture : IDisposable
    {
        private readonly string _root = Path.Combine(Path.GetTempPath(), "VoiceInsertControllerTests", Guid.NewGuid().ToString("N"));

        public SettingsFixture(AppSettings appSettings)
        {
            var paths = new AppPaths(Path.Combine(_root, "app"), Path.Combine(_root, "local"));
            Settings = new SettingsService(paths, new SecretStore(paths));
            Settings.Save(appSettings, "");
        }

        public SettingsService Settings { get; }

        public void Dispose()
        {
            if (Directory.Exists(_root))
            {
                Directory.Delete(_root, true);
            }
        }
    }
}
