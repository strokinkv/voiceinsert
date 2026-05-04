namespace VoiceInsert.App.Services;

public sealed class AppPaths
{
    public AppPaths()
        : this(
            System.IO.Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "VoiceInsert"),
            System.IO.Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "VoiceInsert"))
    {
    }

    public AppPaths(string appDataDirectory, string localDataDirectory)
    {
        AppDataDirectory = appDataDirectory;
        LocalDataDirectory = localDataDirectory;
    }

    public string AppDataDirectory { get; }
    public string LocalDataDirectory { get; }

    public string SettingsPath => System.IO.Path.Combine(AppDataDirectory, "settings.json");
    public string SecretPath => System.IO.Path.Combine(AppDataDirectory, "api-key.dpapi");
    public string LogsDirectory => System.IO.Path.Combine(LocalDataDirectory, "Logs");
}
