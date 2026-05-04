using System.Windows;
using System.Windows.Media;
using System.Windows.Threading;

namespace VoiceInsert.App.Views;

public partial class RecordingOverlayWindow : Window
{
    private const int SampleCount = 56;
    private readonly DispatcherTimer _animationTimer;
    private readonly Queue<float> _samples = new();
    private float _level;
    private float _displayLevel;

    public RecordingOverlayWindow()
    {
        InitializeComponent();
        Left = SystemParameters.WorkArea.Right - Width - 24;
        Top = SystemParameters.WorkArea.Bottom - Height - 48;

        _animationTimer = new DispatcherTimer
        {
            Interval = TimeSpan.FromMilliseconds(34)
        };
        _animationTimer.Tick += (_, _) => AppendSample();
        IsVisibleChanged += (_, _) =>
        {
            if (IsVisible)
            {
                _animationTimer.Start();
                return;
            }

            _animationTimer.Stop();
        };
    }

    public void SetStatus(string status)
    {
        StatusText.Text = status;
    }

    public void Reset()
    {
        _samples.Clear();
        _level = 0;
        _displayLevel = 0;
        RenderWaveform();
    }

    public void SetLevel(float level)
    {
        var normalized = Math.Clamp(level, 0f, 1f);
        var compressed = MathF.Sqrt(normalized);
        _level = Math.Max(_level * 0.45f, compressed);
    }

    private void AppendSample()
    {
        _displayLevel = Math.Max(_level, _displayLevel * 0.72f);
        _samples.Enqueue(_displayLevel);
        _level *= 0.38f;
        while (_samples.Count > SampleCount)
        {
            _samples.Dequeue();
        }

        RenderWaveform();
    }

    private void RenderWaveform()
    {
        var width = Math.Max(1, WaveCanvas.ActualWidth);
        var height = Math.Max(1, WaveCanvas.ActualHeight);
        var centerY = height / 2;
        var samples = _samples.ToArray();
        var points = new PointCollection();
        var mirrorPoints = new PointCollection();
        var fillPoints = new PointCollection();

        if (samples.Length == 0)
        {
            AmplitudeLine.Points = points;
            AmplitudeMirrorLine.Points = mirrorPoints;
            AmplitudeFill.Points = fillPoints;
            return;
        }

        var spacing = width / Math.Max(1, SampleCount - 1);
        var startIndex = SampleCount - samples.Length;

        for (var index = 0; index < samples.Length; index++)
        {
            var normalized = Math.Clamp(samples[index], 0f, 1f);
            var amplitude = normalized < 0.018f ? 0 : Math.Min(normalized * (height * 0.92), height - 4);
            var x = (startIndex + index) * spacing;
            points.Add(new System.Windows.Point(x, centerY - amplitude / 2));
            mirrorPoints.Add(new System.Windows.Point(x, centerY + amplitude / 2));
        }

        foreach (var point in points)
        {
            fillPoints.Add(point);
        }

        for (var index = mirrorPoints.Count - 1; index >= 0; index--)
        {
            fillPoints.Add(mirrorPoints[index]);
        }

        AmplitudeLine.Points = points;
        AmplitudeMirrorLine.Points = mirrorPoints;
        AmplitudeFill.Points = fillPoints;
    }
}
