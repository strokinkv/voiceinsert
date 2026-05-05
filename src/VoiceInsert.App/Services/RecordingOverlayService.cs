using System.Windows;
using VoiceInsert.App.Models;
using VoiceInsert.App.Views;

namespace VoiceInsert.App.Services;

public sealed class RecordingOverlayService(SettingsService settings) : IRecordingOverlayService
{
    private RecordingOverlayWindow? _window;

    public void ShowRecording()
    {
        if (!settings.Current.ShowFloatingRecordingWindow)
        {
            return;
        }

        _window ??= new RecordingOverlayWindow();
        _window.SetStatus(SettingsTexts.For(settings.Current.UiLanguage).OverlayRecording);
        _window.Reset();
        _window.SetLevel(0);
        _window.Show();
    }

    public void SetLevel(float level)
    {
        System.Windows.Application.Current.Dispatcher.Invoke(() => _window?.SetLevel(level));
    }

    public void SetStatus(string status)
    {
        System.Windows.Application.Current.Dispatcher.Invoke(() => _window?.SetStatus(status));
    }

    public void Hide()
    {
        System.Windows.Application.Current.Dispatcher.Invoke(() => _window?.Hide());
    }
}
