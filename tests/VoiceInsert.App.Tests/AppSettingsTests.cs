using VoiceInsert.App.Models;

namespace VoiceInsert.App.Tests;

public sealed class AppSettingsTests
{
    [Fact]
    public void Normalize_FillsDefaultsAndClampsValues()
    {
        var settings = new AppSettings
        {
            BaseUrl = " ",
            Hotkey = "",
            TranslationHotkey = "",
            Temperature = 2.7,
            RequestTimeoutSeconds = 1,
            MaxRecordingSeconds = 0
        };

        settings.Normalize();

        Assert.Equal("http://127.0.0.1:9573", settings.BaseUrl);
        Assert.Equal("Ctrl+Space", settings.Hotkey);
        Assert.Equal("Alt+Y", settings.TranslationHotkey);
        Assert.Equal("wlast", settings.ActiveApiProfile.Name);
        Assert.Equal(["wlast", "groq"], settings.ApiProfiles.Select(profile => profile.Name).ToArray());
        Assert.Equal(AppSettings.CurrentSettingsVersion, settings.SettingsVersion);
        Assert.Equal(1, settings.Temperature);
        Assert.Equal(5, settings.RequestTimeoutSeconds);
        Assert.Equal(1, settings.MaxRecordingSeconds);
    }

    [Fact]
    public void Normalize_MigratesActiveProfileToWlastForVersionFive()
    {
        var settings = new AppSettings
        {
            SettingsVersion = 4,
            ActiveApiProfileId = "groq",
            ApiProfiles =
            [
                new ApiProfile
                {
                    Id = "wlast",
                    Name = "wlast",
                    BaseUrl = "http://127.0.0.1:9573"
                },
                new ApiProfile
                {
                    Id = "groq",
                    Name = "groq",
                    BaseUrl = "https://api.groq.com/openai/",
                    Model = "whisper-large-v3"
                }
            ]
        };

        settings.Normalize();

        Assert.Equal("wlast", settings.ActiveApiProfile.Name);
        Assert.Equal("http://127.0.0.1:9573", settings.BaseUrl);
    }

    [Fact]
    public void Normalize_MigratesOldTranslationHotkeyToAltY()
    {
        var settings = new AppSettings
        {
            SettingsVersion = 5,
            TranslationHotkey = "Ctrl+Alt+E"
        };

        settings.Normalize();

        Assert.Equal("Alt+Y", settings.TranslationHotkey);
    }

    [Fact]
    public void Normalize_AddsGroqProfileWithCorrectDefaultModels()
    {
        var settings = new AppSettings
        {
            ApiProfiles =
            [
                new ApiProfile
                {
                    Id = "wlast",
                    Name = "wlast",
                    BaseUrl = "http://127.0.0.1:9573"
                }
            ],
            ActiveApiProfileId = "wlast"
        };

        settings.Normalize();

        Assert.Equal(["wlast", "groq"], settings.ApiProfiles.Select(profile => profile.Name).ToArray());
        var groq = settings.ApiProfiles.Single(profile => profile.Name == "groq");
        Assert.Equal("https://api.groq.com/openai/", groq.BaseUrl);
        Assert.Equal("whisper-large-v3", groq.Model);
        Assert.Equal("", groq.TranslationModel);
        Assert.Equal("wlast", settings.ActiveApiProfile.Name);
    }
}
