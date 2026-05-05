using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using VoiceInsert.App.Models;

namespace VoiceInsert.App;

public sealed partial class SettingsViewModel : ObservableObject
{
    private readonly App _app;
    private readonly Action<HotkeyCaptureTarget> _startHotkeyCapture;
    private bool _isApplyingApiProfile;

    public SettingsViewModel(App app, Action<HotkeyCaptureTarget> startHotkeyCapture)
    {
        _app = app;
        _startHotkeyCapture = startHotkeyCapture;
        var settings = app.Settings.Current;

        foreach (var profile in settings.ApiProfiles)
        {
            ApiProfiles.Add(CloneProfile(profile));
        }

        SelectedApiProfileId = settings.ActiveApiProfileId;
        ApplyApiProfile(SelectedApiProfileId);
        MicrophoneDeviceId = settings.MicrophoneDeviceId;
        Hotkey = settings.Hotkey;
        TranslationHotkey = settings.TranslationHotkey;
        RecordingMode = settings.RecordingMode;
        SilenceThresholdPercent = settings.SilenceThresholdPercent;
        SilenceTimeoutMilliseconds = settings.SilenceTimeoutMilliseconds;
        MaxRecordingSeconds = settings.MaxRecordingSeconds;
        StartWithWindows = app.Autostart.IsEnabled();
        UiLanguage = settings.UiLanguage;
        LaunchMinimizedToTray = settings.LaunchMinimizedToTray;
        ShowFloatingRecordingWindow = settings.ShowFloatingRecordingWindow;
        EnableSounds = settings.EnableSounds;
        RestoreClipboardContent = settings.RestoreClipboardContent;
        DelayBeforePasteMilliseconds = settings.DelayBeforePasteMilliseconds;
        DelayBeforeClipboardRestoreMilliseconds = settings.DelayBeforeClipboardRestoreMilliseconds;
        LogLevel = settings.LogLevel;
        ApiKey = app.Settings.ApiKey;
        app.LastError.Changed += (_, _) =>
        {
            System.Windows.Application.Current.Dispatcher.Invoke(() =>
            {
                OnPropertyChanged(nameof(LastErrorTime));
                OnPropertyChanged(nameof(LastErrorMessage));
            });
        };
        RefreshLocalizedItems();
        RefreshDevices();
    }

    public ObservableCollection<string> Models { get; } = [];
    public ObservableCollection<ApiProfile> ApiProfiles { get; } = [];
    public ObservableCollection<string> MicrophoneDevices { get; } = [];
    public ObservableCollection<EnumDisplayItem<AppLanguage>> UiLanguageItems { get; } = [];
    public ObservableCollection<EnumDisplayItem<RecordingMode>> RecordingModeItems { get; } = [];
    public IReadOnlyList<double> TemperatureOptions { get; } =
        Enumerable.Range(0, 11).Select(value => value / 10d).ToArray();
    public IReadOnlyList<string> LogLevels { get; } = ["Information", "Debug", "Warning", "Error"];
    public SettingsTexts Texts => SettingsTexts.For(UiLanguage);

    [ObservableProperty] private string _baseUrl = "";
    [ObservableProperty] private string _apiKey = "";
    [ObservableProperty] private string _selectedApiProfileId = "";
    [ObservableProperty] private string _apiProfileName = "";
    [ObservableProperty] private string _model = "";
    [ObservableProperty] private string _language = "";
    [ObservableProperty] private string _prompt = "";
    [ObservableProperty] private string _translationPrompt = "";
    [ObservableProperty] private double _temperature;
    [ObservableProperty] private int _requestTimeoutSeconds;
    [ObservableProperty] private string _microphoneDeviceId = "";
    [ObservableProperty] private string _hotkey = "Ctrl+Space";
    [ObservableProperty] private string _translationHotkey = "Alt+Y";
    [ObservableProperty] private RecordingMode _recordingMode;
    [ObservableProperty] private int _silenceThresholdPercent;
    [ObservableProperty] private int _silenceTimeoutMilliseconds;
    [ObservableProperty] private int _maxRecordingSeconds;
    [ObservableProperty] private bool _startWithWindows;
    [ObservableProperty] private AppLanguage _uiLanguage;
    [ObservableProperty] private bool _launchMinimizedToTray;
    [ObservableProperty] private bool _showFloatingRecordingWindow;
    [ObservableProperty] private bool _enableSounds;
    [ObservableProperty] private bool _restoreClipboardContent;
    [ObservableProperty] private int _delayBeforePasteMilliseconds;
    [ObservableProperty] private int _delayBeforeClipboardRestoreMilliseconds;
    [ObservableProperty] private string _logLevel = "Information";
    [ObservableProperty] private string _statusMessage = "";
    [ObservableProperty] private int _selectedSettingsPage;
    [ObservableProperty] private string _microphoneStatus = "";

    public string CurrentPageTitle => SelectedSettingsPage switch
    {
        0 => Texts.General,
        1 => Texts.Hotkey,
        2 => Texts.Audio,
        3 => Texts.TranscriptionApi,
        4 => Texts.Insertion,
        5 => Texts.Sounds,
        6 => Texts.LogsDiagnostics,
        _ => Texts.General
    };

    public string LastErrorTime =>
        _app.LastError.OccurredAt?.ToString("yyyy-MM-dd HH:mm:ss") ?? Texts.NoErrorsSinceStart;

    public string LastErrorMessage =>
        string.IsNullOrWhiteSpace(_app.LastError.Message) ? "" : _app.LastError.Message;

    [RelayCommand]
    private void Save()
    {
        SaveCurrentApiFieldsToSelectedProfile();
        var settings = new AppSettings
        {
            SettingsVersion = AppSettings.CurrentSettingsVersion,
            ActiveApiProfileId = SelectedApiProfileId,
            ApiProfiles = ApiProfiles.Select(CloneProfile).ToList(),
            BaseUrl = BaseUrl,
            Model = Model,
            Language = Language,
            Prompt = Prompt,
            TranslationPrompt = TranslationPrompt,
            Temperature = Temperature,
            RequestTimeoutSeconds = RequestTimeoutSeconds,
            MicrophoneDeviceId = MicrophoneDeviceId,
            Hotkey = Hotkey,
            TranslationHotkey = TranslationHotkey,
            RecordingMode = RecordingMode,
            SilenceThresholdPercent = SilenceThresholdPercent,
            SilenceTimeoutMilliseconds = SilenceTimeoutMilliseconds,
            MaxRecordingSeconds = MaxRecordingSeconds,
            StartWithWindows = StartWithWindows,
            UiLanguage = UiLanguage,
            LaunchMinimizedToTray = LaunchMinimizedToTray,
            ShowFloatingRecordingWindow = ShowFloatingRecordingWindow,
            EnableSounds = EnableSounds,
            RestoreClipboardContent = RestoreClipboardContent,
            DelayBeforePasteMilliseconds = DelayBeforePasteMilliseconds,
            DelayBeforeClipboardRestoreMilliseconds = DelayBeforeClipboardRestoreMilliseconds,
            LogLevel = LogLevel
        };

        _app.Settings.Save(settings, ApiKey);
        _app.Autostart.SetEnabled(StartWithWindows);
        _app.Logger.Configure(_app.Settings.Current.LogLevel);
        StatusMessage = Texts.Saved;
    }

    [RelayCommand]
    private void RefreshDevices()
    {
        MicrophoneDevices.Clear();

        foreach (var device in _app.Recorder.GetInputDevices())
        {
            MicrophoneDevices.Add(device);
        }

        if (string.IsNullOrWhiteSpace(MicrophoneDeviceId) && MicrophoneDevices.Count > 0)
        {
            MicrophoneDeviceId = MicrophoneDevices[0];
        }
    }

    [RelayCommand]
    public async Task LoadModelsAsync()
    {
        try
        {
            SaveCurrentApiFieldsToSelectedProfile();
            Models.Clear();
            foreach (var model in await _app.ModelsClient.GetModelsAsync(
                BaseUrl,
                ApiKey,
                CancellationToken.None))
            {
                Models.Add(model.Id);
            }

            if (string.IsNullOrWhiteSpace(Model) && Models.Count > 0)
            {
                Model = Models[0];
            }

            StatusMessage = Models.Count > 0
                ? string.Format(Texts.ApiStatusConnected, Models.Count)
                : Texts.NoModelsReturned;
        }
        catch (Exception exception)
        {
            StatusMessage = Texts.FailedToLoadModels;
            _app.LastError.Set(exception);
            OnPropertyChanged(nameof(LastErrorTime));
            OnPropertyChanged(nameof(LastErrorMessage));
        }
    }

    [RelayCommand]
    private async Task TestApiConnectionAsync()
    {
        try
        {
            SaveCurrentApiFieldsToSelectedProfile();
            var selected = FindSelectedApiProfile();
            if (selected is null)
            {
                StatusMessage = Texts.FailedToLoadModels;
                return;
            }

            var result = await _app.ApiProfileHealthCheck.CheckAsync(
                selected,
                ApiKey,
                CancellationToken.None);
            StatusMessage = result.Message;
            if (!result.IsHealthy)
            {
                _app.LastError.Set(result.Message);
                OnPropertyChanged(nameof(LastErrorTime));
                OnPropertyChanged(nameof(LastErrorMessage));
            }
        }
        catch (Exception exception)
        {
            StatusMessage = Texts.FailedToLoadModels;
            _app.LastError.Set(exception);
            OnPropertyChanged(nameof(LastErrorTime));
            OnPropertyChanged(nameof(LastErrorMessage));
        }
    }

    [RelayCommand]
    private void AddApiProfile()
    {
        SaveCurrentApiFieldsToSelectedProfile();
        var profile = new ApiProfile
        {
            Name = $"Profile {ApiProfiles.Count + 1}",
            BaseUrl = BaseUrl,
            Model = Model,
            Language = Language,
            Prompt = Prompt,
            TranslationPrompt = TranslationPrompt,
            Temperature = Temperature,
            RequestTimeoutSeconds = RequestTimeoutSeconds
        };
        ApiProfiles.Add(profile);
        SelectedApiProfileId = profile.Id;
        StatusMessage = Texts.Saved;
    }

    [RelayCommand]
    private void DeleteApiProfile()
    {
        if (ApiProfiles.Count <= 1)
        {
            return;
        }

        var selected = FindSelectedApiProfile();
        if (selected is null)
        {
            return;
        }

        var index = ApiProfiles.IndexOf(selected);
        ApiProfiles.Remove(selected);
        _app.Settings.ApiKeys.Remove(selected.Id);
        SelectedApiProfileId = ApiProfiles[Math.Clamp(index - 1, 0, ApiProfiles.Count - 1)].Id;
    }

    [RelayCommand]
    private async Task TestMicrophoneAsync()
    {
        try
        {
            var level = await _app.Recorder.MeasureInputLevelAsync(TimeSpan.FromMilliseconds(900));
            MicrophoneStatus = string.Format(Texts.MicrophoneLevel, Math.Round(level * 100));
        }
        catch (Exception exception)
        {
            MicrophoneStatus = Texts.MicrophoneTestFailed;
            _app.LastError.Set(exception);
            OnPropertyChanged(nameof(LastErrorTime));
            OnPropertyChanged(nameof(LastErrorMessage));
        }
    }

    [RelayCommand]
    private void ChangeHotkey()
    {
        _startHotkeyCapture(HotkeyCaptureTarget.Transcription);
    }

    [RelayCommand]
    private void ChangeTranslationHotkey()
    {
        _startHotkeyCapture(HotkeyCaptureTarget.Translation);
    }

    [RelayCommand]
    private void TestStartRecordSound() => _app.Sounds.PlayStart();

    [RelayCommand]
    private void TestStopRecordSound() => _app.Sounds.PlayStop();

    [RelayCommand]
    private void TestErrorSound() => _app.Sounds.PlayError();

    [RelayCommand]
    private void OpenLogsFolder()
    {
        Directory.CreateDirectory(_app.Paths.LogsDirectory);
        Process.Start(new ProcessStartInfo
        {
            FileName = _app.Paths.LogsDirectory,
            UseShellExecute = true
        });
    }

    [RelayCommand]
    private void ClearLogs()
    {
        if (!Directory.Exists(_app.Paths.LogsDirectory))
        {
            return;
        }

        foreach (var file in Directory.EnumerateFiles(_app.Paths.LogsDirectory, "*.log"))
        {
            try
            {
                File.Delete(file);
            }
            catch
            {
                StatusMessage = Texts.SomeLogFilesLocked;
            }
        }
    }

    partial void OnUiLanguageChanged(AppLanguage value)
    {
        OnPropertyChanged(nameof(Texts));
        RefreshLocalizedItems();
        OnPropertyChanged(nameof(LastErrorTime));
    }

    partial void OnSelectedApiProfileIdChanging(string value)
    {
        if (!_isApplyingApiProfile)
        {
            SaveCurrentApiFieldsToSelectedProfile();
        }
    }

    partial void OnSelectedApiProfileIdChanged(string value)
    {
        ApplyApiProfile(value);
    }

    partial void OnApiProfileNameChanged(string value)
    {
        if (_isApplyingApiProfile)
        {
            return;
        }

        var selected = FindSelectedApiProfile();
        if (selected is not null)
        {
            selected.Name = string.IsNullOrWhiteSpace(value) ? "wlast" : value.Trim();
        }
    }

    private void ApplyApiProfile(string profileId)
    {
        var profile = ApiProfiles.FirstOrDefault(item => item.Id == profileId) ?? ApiProfiles.FirstOrDefault();
        if (profile is null)
        {
            return;
        }

        _isApplyingApiProfile = true;
        try
        {
            BaseUrl = profile.BaseUrl;
            Model = profile.Model;
            Language = profile.Language;
            Prompt = profile.Prompt;
            TranslationPrompt = profile.TranslationPrompt;
            Temperature = profile.Temperature;
            RequestTimeoutSeconds = profile.RequestTimeoutSeconds;
            ApiProfileName = profile.Name;
            ApiKey = _app.Settings.GetApiKey(profile.Id);
            Models.Clear();
        }
        finally
        {
            _isApplyingApiProfile = false;
        }
    }

    private void SaveCurrentApiFieldsToSelectedProfile()
    {
        var profile = FindSelectedApiProfile();
        if (profile is null)
        {
            return;
        }

        profile.Name = string.IsNullOrWhiteSpace(ApiProfileName) ? profile.Name : ApiProfileName.Trim();
        profile.BaseUrl = BaseUrl;
        profile.Model = Model;
        profile.TranslationModel = "";
        profile.Language = Language;
        profile.Prompt = Prompt;
        profile.TranslationPrompt = TranslationPrompt;
        profile.Temperature = Temperature;
        profile.RequestTimeoutSeconds = RequestTimeoutSeconds;
        profile.Normalize();
        if (string.IsNullOrWhiteSpace(ApiKey))
        {
            _app.Settings.ApiKeys.Remove(profile.Id);
        }
        else
        {
            _app.Settings.ApiKeys[profile.Id] = ApiKey;
        }
    }

    private ApiProfile? FindSelectedApiProfile()
    {
        return ApiProfiles.FirstOrDefault(profile => profile.Id == SelectedApiProfileId);
    }

    private static ApiProfile CloneProfile(ApiProfile profile)
    {
        return new ApiProfile
        {
            Id = profile.Id,
            Name = profile.Name,
            BaseUrl = profile.BaseUrl,
            Model = profile.Model,
            TranslationModel = "",
            Language = profile.Language,
            Prompt = profile.Prompt,
            TranslationPrompt = profile.TranslationPrompt,
            Temperature = profile.Temperature,
            RequestTimeoutSeconds = profile.RequestTimeoutSeconds
        };
    }

    private void RefreshLocalizedItems()
    {
        SetItems(UiLanguageItems,
        [
            new EnumDisplayItem<AppLanguage>(AppLanguage.Russian, Texts.LanguageRussian),
            new EnumDisplayItem<AppLanguage>(AppLanguage.English, Texts.LanguageEnglish)
        ]);

        SetItems(RecordingModeItems,
        [
            new EnumDisplayItem<RecordingMode>(RecordingMode.Toggle, Texts.RecordingModeToggle),
            new EnumDisplayItem<RecordingMode>(RecordingMode.Hold, Texts.RecordingModeHold),
            new EnumDisplayItem<RecordingMode>(RecordingMode.SilenceTimeout, Texts.RecordingModeSilenceTimeout)
        ]);
    }

    private static void SetItems<T>(
        ObservableCollection<EnumDisplayItem<T>> target,
        IReadOnlyList<EnumDisplayItem<T>> source)
        where T : struct, Enum
    {
        if (target.Count == 0)
        {
            foreach (var item in source)
            {
                target.Add(item);
            }

            return;
        }

        foreach (var sourceItem in source)
        {
            var existing = target.FirstOrDefault(item =>
                EqualityComparer<T>.Default.Equals(item.Value, sourceItem.Value));
            if (existing is not null)
            {
                existing.DisplayName = sourceItem.DisplayName;
            }
        }
    }
}
