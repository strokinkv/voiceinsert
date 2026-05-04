using VoiceInsert.App.Models;
using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class SettingsServiceTests
{
    [Fact]
    public void Load_NormalizesLegacySettings()
    {
        using var fixture = new SettingsFixture();
        Directory.CreateDirectory(fixture.Paths.AppDataDirectory);
        File.WriteAllText(fixture.Paths.SettingsPath, """{"baseUrl":"","temperature":7,"hotkey":"","translationHotkey":""}""");

        fixture.Settings.Load();

        Assert.Equal(AppSettings.CurrentSettingsVersion, fixture.Settings.Current.SettingsVersion);
        Assert.Equal("http://127.0.0.1:9573", fixture.Settings.Current.BaseUrl);
        Assert.Equal("Ctrl+Space", fixture.Settings.Current.Hotkey);
        Assert.Equal("Alt+Y", fixture.Settings.Current.TranslationHotkey);
        Assert.Equal("wlast", fixture.Settings.Current.ActiveApiProfile.Name);
        Assert.Equal(1, fixture.Settings.Current.Temperature);
    }

    private sealed class SettingsFixture : IDisposable
    {
        private readonly string _root = Path.Combine(Path.GetTempPath(), "VoiceInsertSettingsTests", Guid.NewGuid().ToString("N"));

        public SettingsFixture()
        {
            Paths = new AppPaths(Path.Combine(_root, "app"), Path.Combine(_root, "local"));
            Settings = new SettingsService(Paths, new SecretStore(Paths));
        }

        public AppPaths Paths { get; }
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
