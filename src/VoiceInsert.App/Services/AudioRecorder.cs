using NAudio.CoreAudioApi;
using NAudio.Wave;
using System.IO;

namespace VoiceInsert.App.Services;

public sealed class AudioRecorder(SettingsService settings) : IDisposable
{
    private WaveInEvent? _waveIn;
    private WaveFileWriter? _writer;
    private MemoryStream? _buffer;
    private TaskCompletionSource? _recordingStopped;

    public event EventHandler<float>? LevelChanged;

    public bool IsRecording => _waveIn is not null;

    public IReadOnlyList<string> GetInputDevices()
    {
        var friendlyNames = GetFriendlyCaptureNames();
        return Enumerable.Range(0, WaveIn.DeviceCount)
            .Select(index => FormatDeviceName(
                index,
                index < friendlyNames.Count ? friendlyNames[index] : WaveIn.GetCapabilities(index).ProductName))
            .ToArray();
    }

    public async Task<float> MeasureInputLevelAsync(TimeSpan duration)
    {
        var deviceNumber = ResolveDeviceNumber();
        var maxLevel = 0f;
        using var waveIn = new WaveInEvent
        {
            DeviceNumber = deviceNumber,
            WaveFormat = new WaveFormat(16000, 16, 1),
            BufferMilliseconds = 50
        };

        waveIn.DataAvailable += (_, e) =>
        {
            maxLevel = Math.Max(maxLevel, CalculatePeakLevel(e.Buffer, e.BytesRecorded));
        };

        waveIn.StartRecording();
        try
        {
            await Task.Delay(duration);
        }
        finally
        {
            waveIn.StopRecording();
        }

        return maxLevel;
    }

    public void Start()
    {
        if (_waveIn is not null)
        {
            return;
        }

        var deviceNumber = ResolveDeviceNumber();
        _buffer = new MemoryStream();
        _waveIn = new WaveInEvent
        {
            DeviceNumber = deviceNumber,
            WaveFormat = new WaveFormat(16000, 16, 1),
            BufferMilliseconds = 50
        };
        _writer = new WaveFileWriter(new IgnoreDisposeStream(_buffer), _waveIn.WaveFormat);
        _recordingStopped = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        _waveIn.DataAvailable += OnDataAvailable;
        _waveIn.RecordingStopped += OnRecordingStopped;
        _waveIn.StartRecording();
    }

    public async Task<byte[]> StopAsync()
    {
        if (_waveIn is null || _writer is null || _buffer is null)
        {
            return [];
        }

        try
        {
            _waveIn.StopRecording();
            if (_recordingStopped is not null)
            {
                await _recordingStopped.Task.WaitAsync(TimeSpan.FromSeconds(3));
            }
        }
        catch (TimeoutException)
        {
            // Some drivers do not raise RecordingStopped reliably. The WAV buffer is still usable
            // after flushing the writer, so continue instead of losing the transcription.
        }

        _writer.Flush();
        _writer.Dispose();
        _writer = null;

        var bytes = _buffer.ToArray();
        _buffer.Dispose();
        _buffer = null;

        _waveIn.Dispose();
        _waveIn = null;
        _recordingStopped = null;

        return bytes;
    }

    private int ResolveDeviceNumber()
    {
        if (string.IsNullOrWhiteSpace(settings.Current.MicrophoneDeviceId))
        {
            return 0;
        }

        if (TryParseDeviceNumber(settings.Current.MicrophoneDeviceId, out var parsedDeviceNumber)
            && parsedDeviceNumber >= 0
            && parsedDeviceNumber < WaveIn.DeviceCount)
        {
            return parsedDeviceNumber;
        }

        for (var index = 0; index < WaveIn.DeviceCount; index++)
        {
            var shortName = WaveIn.GetCapabilities(index).ProductName;
            if (shortName == settings.Current.MicrophoneDeviceId
                || settings.Current.MicrophoneDeviceId.Contains(shortName, StringComparison.OrdinalIgnoreCase))
            {
                return index;
            }
        }

        return 0;
    }

    private static string FormatDeviceName(int index, string name) => $"[{index}] {name}";

    private static bool TryParseDeviceNumber(string deviceName, out int deviceNumber)
    {
        deviceNumber = 0;
        if (!deviceName.StartsWith("[", StringComparison.Ordinal))
        {
            return false;
        }

        var end = deviceName.IndexOf(']', StringComparison.Ordinal);
        return end > 1 && int.TryParse(deviceName[1..end], out deviceNumber);
    }

    private static IReadOnlyList<string> GetFriendlyCaptureNames()
    {
        try
        {
            using var enumerator = new MMDeviceEnumerator();
            return enumerator
                .EnumerateAudioEndPoints(DataFlow.Capture, DeviceState.Active)
                .Select(device => device.FriendlyName)
                .ToArray();
        }
        catch
        {
            return [];
        }
    }

    private void OnDataAvailable(object? sender, WaveInEventArgs e)
    {
        _writer?.Write(e.Buffer, 0, e.BytesRecorded);
        _writer?.Flush();

        LevelChanged?.Invoke(this, CalculatePeakLevel(e.Buffer, e.BytesRecorded));
    }

    private static float CalculatePeakLevel(byte[] buffer, int bytesRecorded)
    {
        var max = 0;
        for (var index = 0; index < bytesRecorded; index += 2)
        {
            var sample = BitConverter.ToInt16(buffer, index);
            max = Math.Max(max, Math.Abs((int)sample));
        }

        return Math.Clamp(max / 32768f, 0f, 1f);
    }

    private void OnRecordingStopped(object? sender, StoppedEventArgs e)
    {
        if (_waveIn is not null)
        {
            _waveIn.DataAvailable -= OnDataAvailable;
            _waveIn.RecordingStopped -= OnRecordingStopped;
        }

        _recordingStopped?.TrySetResult();
    }

    public void Dispose()
    {
        _waveIn?.Dispose();
        _writer?.Dispose();
        _buffer?.Dispose();
    }

    private sealed class IgnoreDisposeStream(Stream inner) : Stream
    {
        public override bool CanRead => inner.CanRead;
        public override bool CanSeek => inner.CanSeek;
        public override bool CanWrite => inner.CanWrite;
        public override long Length => inner.Length;
        public override long Position { get => inner.Position; set => inner.Position = value; }
        public override void Flush() => inner.Flush();
        public override int Read(byte[] buffer, int offset, int count) => inner.Read(buffer, offset, count);
        public override long Seek(long offset, SeekOrigin origin) => inner.Seek(offset, origin);
        public override void SetLength(long value) => inner.SetLength(value);
        public override void Write(byte[] buffer, int offset, int count) => inner.Write(buffer, offset, count);
        protected override void Dispose(bool disposing) { }
    }
}
