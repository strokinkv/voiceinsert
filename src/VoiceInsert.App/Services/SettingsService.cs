using System.Text.Json;
using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

public sealed class SettingsService(AppPaths paths, SecretStore secretStore)
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web)
    {
        WriteIndented = true
    };

    public AppSettings Current { get; private set; } = new();
    public Dictionary<string, string> ApiKeys { get; private set; } = [];
    public string ApiKey => GetApiKey(Current.ActiveApiProfileId);

    public void Load()
    {
        Directory.CreateDirectory(paths.AppDataDirectory);

        if (File.Exists(paths.SettingsPath))
        {
            var json = File.ReadAllText(paths.SettingsPath);
            Current = JsonSerializer.Deserialize<AppSettings>(json, JsonOptions) ?? new AppSettings();
        }

        Current.Normalize();

        ApiKeys = secretStore.LoadApiKeys().ToDictionary();
        if (ApiKeys.TryGetValue("default", out var legacyApiKey)
            && !ApiKeys.ContainsKey(Current.ActiveApiProfileId))
        {
            ApiKeys[Current.ActiveApiProfileId] = legacyApiKey;
            ApiKeys.Remove("default");
            secretStore.SaveApiKeys(ApiKeys);
        }
    }

    public void Save(AppSettings settings, string apiKey)
    {
        Directory.CreateDirectory(paths.AppDataDirectory);
        settings.UpdateActiveProfileFromFlatSettings();
        settings.Normalize();
        Current = settings;
        if (string.IsNullOrWhiteSpace(apiKey))
        {
            ApiKeys.Remove(Current.ActiveApiProfileId);
        }
        else
        {
            ApiKeys[Current.ActiveApiProfileId] = apiKey;
        }

        File.WriteAllText(paths.SettingsPath, JsonSerializer.Serialize(settings, JsonOptions));
        secretStore.SaveApiKeys(ApiKeys);
    }

    public string GetApiKey(string profileId)
    {
        return ApiKeys.TryGetValue(profileId, out var apiKey) ? apiKey : "";
    }
}
