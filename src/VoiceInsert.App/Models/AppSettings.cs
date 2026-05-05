using VoiceInsert.App.Services;

namespace VoiceInsert.App.Models;

public sealed class AppSettings
{
    public const int CurrentSettingsVersion = 7;

    public int SettingsVersion { get; set; } = CurrentSettingsVersion;
    public string BaseUrl { get; set; } = "http://127.0.0.1:9573";
    public string Model { get; set; } = "";
    public string TranslationModel { get; set; } = "";
    public string Language { get; set; } = "";
    public string Prompt { get; set; } = "";
    public string TranslationPrompt { get; set; } = "";
    public double Temperature { get; set; } = 0.2;
    public int RequestTimeoutSeconds { get; set; } = 120;
    public string MicrophoneDeviceId { get; set; } = "";
    public string Hotkey { get; set; } = "Ctrl+Space";
    public string TranslationHotkey { get; set; } = "Alt+Y";
    public RecordingMode RecordingMode { get; set; } = RecordingMode.Toggle;
    public int SilenceThresholdPercent { get; set; } = 4;
    public int SilenceTimeoutMilliseconds { get; set; } = 1200;
    public int MaxRecordingSeconds { get; set; } = 120;
    public bool StartWithWindows { get; set; }
    public AppLanguage UiLanguage { get; set; } = AppLanguage.Russian;
    public bool LaunchMinimizedToTray { get; set; } = true;
    public bool ShowFloatingRecordingWindow { get; set; } = true;
    public bool EnableSounds { get; set; } = true;
    public bool RestoreClipboardContent { get; set; } = true;
    public int DelayBeforePasteMilliseconds { get; set; } = 80;
    public int DelayBeforeClipboardRestoreMilliseconds { get; set; } = 300;
    public string LogLevel { get; set; } = "Information";
    public string ActiveApiProfileId { get; set; } = "";
    public List<ApiProfile> ApiProfiles { get; set; } = [];

    public void Normalize()
    {
        var originalSettingsVersion = SettingsVersion;
        SettingsVersion = CurrentSettingsVersion;
        BaseUrl = string.IsNullOrWhiteSpace(BaseUrl) ? "http://127.0.0.1:9573" : BaseUrl.Trim();
        Hotkey = string.IsNullOrWhiteSpace(Hotkey) ? "Ctrl+Space" : Hotkey.Trim();
        TranslationHotkey = string.IsNullOrWhiteSpace(TranslationHotkey) ? "Alt+Y" : TranslationHotkey.Trim();
        if (originalSettingsVersion < 6
            && TranslationHotkey.Equals("Ctrl+Alt+E", StringComparison.OrdinalIgnoreCase))
        {
            TranslationHotkey = "Alt+Y";
        }

        Hotkey = HotkeyMatcher.TryNormalize(Hotkey, out var normalizedHotkey) ? normalizedHotkey : "Ctrl+Space";
        TranslationHotkey = HotkeyMatcher.TryNormalize(TranslationHotkey, out var normalizedTranslationHotkey)
            ? normalizedTranslationHotkey
            : "Alt+Y";
        if (TranslationHotkey.Equals(Hotkey, StringComparison.OrdinalIgnoreCase))
        {
            TranslationHotkey = Hotkey.Equals("Alt+Y", StringComparison.OrdinalIgnoreCase)
                ? "Ctrl+Alt+Y"
                : "Alt+Y";
        }
        Temperature = Math.Clamp(Math.Round(Temperature, 1), 0, 1);
        RequestTimeoutSeconds = Math.Clamp(RequestTimeoutSeconds, 5, 600);
        SilenceThresholdPercent = Math.Clamp(SilenceThresholdPercent, 0, 100);
        SilenceTimeoutMilliseconds = Math.Clamp(SilenceTimeoutMilliseconds, 100, 30000);
        MaxRecordingSeconds = Math.Clamp(MaxRecordingSeconds, 1, 3600);
        DelayBeforePasteMilliseconds = Math.Clamp(DelayBeforePasteMilliseconds, 0, 5000);
        DelayBeforeClipboardRestoreMilliseconds = Math.Clamp(DelayBeforeClipboardRestoreMilliseconds, 0, 30000);
        LogLevel = string.IsNullOrWhiteSpace(LogLevel) ? "Information" : LogLevel;
        EnsureApiProfiles();
        if (originalSettingsVersion < 5)
        {
            ActiveApiProfileId = ApiProfiles.First(profile =>
                profile.Name.Equals("wlast", StringComparison.OrdinalIgnoreCase)).Id;
        }

        ApplyActiveProfileToFlatSettings();
    }

    public ApiProfile ActiveApiProfile => ApiProfiles.First(profile => profile.Id == ActiveApiProfileId);

    public void UpdateActiveProfileFromFlatSettings()
    {
        EnsureApiProfiles();
        var profile = ActiveApiProfile;
        profile.BaseUrl = BaseUrl;
        profile.Model = Model;
        profile.TranslationModel = "";
        profile.Language = Language;
        profile.Prompt = Prompt;
        profile.TranslationPrompt = TranslationPrompt;
        profile.Temperature = Temperature;
        profile.RequestTimeoutSeconds = RequestTimeoutSeconds;
        profile.Normalize();
    }

    private void EnsureApiProfiles()
    {
        if (ApiProfiles.Count == 0)
        {
            ApiProfiles.Add(CreateWlastProfile(BaseUrl, Model, Language, Prompt, TranslationPrompt, Temperature, RequestTimeoutSeconds));
        }

        foreach (var profile in ApiProfiles)
        {
            profile.Normalize();
            if (profile.Name.Equals("Default", StringComparison.OrdinalIgnoreCase)
                && profile.BaseUrl.Equals("http://127.0.0.1:9573", StringComparison.OrdinalIgnoreCase))
            {
                profile.Name = "wlast";
            }
        }

        EnsureWlastProfile();
        EnsureGroqProfile();
        ApiProfiles = ApiProfiles
            .OrderBy(profile => GetProfileOrder(profile))
            .ThenBy(profile => profile.Name, StringComparer.OrdinalIgnoreCase)
            .ToList();

        if (string.IsNullOrWhiteSpace(ActiveApiProfileId)
            || ApiProfiles.All(profile => profile.Id != ActiveApiProfileId))
        {
            ActiveApiProfileId = ApiProfiles.First(profile => profile.Name.Equals("wlast", StringComparison.OrdinalIgnoreCase)).Id;
        }
    }

    private void EnsureWlastProfile()
    {
        if (ApiProfiles.Any(profile => profile.Name.Equals("wlast", StringComparison.OrdinalIgnoreCase)))
        {
            return;
        }

        ApiProfiles.Insert(0, CreateWlastProfile(
            "http://127.0.0.1:9573",
            "",
            Language,
            Prompt,
            TranslationPrompt,
            Temperature,
            RequestTimeoutSeconds));
    }

    private void EnsureGroqProfile()
    {
        if (ApiProfiles.Any(profile => profile.Name.Equals("groq", StringComparison.OrdinalIgnoreCase)
            || profile.BaseUrl.Contains("groq.com", StringComparison.OrdinalIgnoreCase)))
        {
            return;
        }

        ApiProfiles.Add(new ApiProfile
        {
            Name = "groq",
            BaseUrl = "https://api.groq.com/openai/",
            Model = "whisper-large-v3",
            Temperature = 0.2,
            RequestTimeoutSeconds = 120
        });
    }

    private static ApiProfile CreateWlastProfile(
        string baseUrl,
        string model,
        string language,
        string prompt,
        string translationPrompt,
        double temperature,
        int requestTimeoutSeconds)
    {
        return new ApiProfile
        {
            Name = "wlast",
            BaseUrl = string.IsNullOrWhiteSpace(baseUrl) ? "http://127.0.0.1:9573" : baseUrl,
            Model = model,
            Language = language,
            Prompt = prompt,
            TranslationPrompt = translationPrompt,
            Temperature = temperature,
            RequestTimeoutSeconds = requestTimeoutSeconds
        };
    }

    private static int GetProfileOrder(ApiProfile profile)
    {
        if (profile.Name.Equals("wlast", StringComparison.OrdinalIgnoreCase))
        {
            return 0;
        }

        if (profile.Name.Equals("groq", StringComparison.OrdinalIgnoreCase)
            || profile.BaseUrl.Contains("groq.com", StringComparison.OrdinalIgnoreCase))
        {
            return 1;
        }

        return 2;
    }

    private void ApplyActiveProfileToFlatSettings()
    {
        var profile = ActiveApiProfile;
        BaseUrl = profile.BaseUrl;
        Model = profile.Model;
        TranslationModel = "";
        Language = profile.Language;
        Prompt = profile.Prompt;
        TranslationPrompt = profile.TranslationPrompt;
        Temperature = profile.Temperature;
        RequestTimeoutSeconds = profile.RequestTimeoutSeconds;
    }
}
