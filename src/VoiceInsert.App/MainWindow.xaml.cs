using System.Windows;
using VoiceInsert.App.Models;

namespace VoiceInsert.App;

public partial class MainWindow : Window
{
    private readonly SettingsViewModel _viewModel;
    private HotkeyCaptureTarget? _captureTarget;

    public MainWindow()
    {
        InitializeComponent();
        _viewModel = new SettingsViewModel((App)System.Windows.Application.Current, StartHotkeyCapture);
        DataContext = _viewModel;
        ApiKeyBox.Password = _viewModel.ApiKey;
        _viewModel.PropertyChanged += (_, args) =>
        {
            if (args.PropertyName == nameof(SettingsViewModel.ApiKey) && ApiKeyBox.Password != _viewModel.ApiKey)
            {
                ApiKeyBox.Password = _viewModel.ApiKey;
            }
        };
    }

    private void CloseClicked(object sender, RoutedEventArgs e)
    {
        Hide();
    }

    private void SaveClicked(object sender, RoutedEventArgs e)
    {
        _viewModel.ApiKey = ApiKeyBox.Password;
        _viewModel.SaveCommand.Execute(null);
    }

    private void ApiKeyChanged(object sender, RoutedEventArgs e)
    {
        if (_viewModel.ApiKey != ApiKeyBox.Password)
        {
            _viewModel.ApiKey = ApiKeyBox.Password;
        }
    }

    private void StartHotkeyCapture(HotkeyCaptureTarget target)
    {
        _captureTarget = target;
        _viewModel.StatusMessage = _viewModel.Texts.PressNewHotkey;
        Activate();
        Focus();
    }

    private void WindowPreviewKeyDown(object sender, System.Windows.Input.KeyEventArgs e)
    {
        if (_captureTarget is null)
        {
            return;
        }

        e.Handled = true;
        var key = e.Key == System.Windows.Input.Key.System ? e.SystemKey : e.Key;
        if (key is System.Windows.Input.Key.LeftCtrl
            or System.Windows.Input.Key.RightCtrl
            or System.Windows.Input.Key.LeftAlt
            or System.Windows.Input.Key.RightAlt
            or System.Windows.Input.Key.LeftShift
            or System.Windows.Input.Key.RightShift
            or System.Windows.Input.Key.LWin
            or System.Windows.Input.Key.RWin)
        {
            return;
        }

        var modifiers = System.Windows.Input.Keyboard.Modifiers;
        var parts = new List<string>();
        if ((modifiers & System.Windows.Input.ModifierKeys.Control) != 0)
        {
            parts.Add("Ctrl");
        }

        if ((modifiers & System.Windows.Input.ModifierKeys.Alt) != 0)
        {
            parts.Add("Alt");
        }

        if ((modifiers & System.Windows.Input.ModifierKeys.Shift) != 0)
        {
            parts.Add("Shift");
        }

        if ((modifiers & System.Windows.Input.ModifierKeys.Windows) != 0)
        {
            parts.Add("Win");
        }

        parts.Add(key.ToString());
        var hotkey = string.Join("+", parts);
        if (_captureTarget == HotkeyCaptureTarget.Translation)
        {
            _viewModel.TranslationHotkey = hotkey;
        }
        else
        {
            _viewModel.Hotkey = hotkey;
        }

        _viewModel.StatusMessage = _viewModel.Texts.HotkeyCaptured;
        _captureTarget = null;
    }

    protected override void OnClosing(System.ComponentModel.CancelEventArgs e)
    {
        if (((App)System.Windows.Application.Current).IsShuttingDown)
        {
            return;
        }

        e.Cancel = true;
        Hide();
    }

    private async void WindowLoaded(object sender, RoutedEventArgs e)
    {
        await _viewModel.LoadModelsAsync();
    }
}
