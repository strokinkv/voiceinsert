using System.Security.Cryptography;
using System.Text;

namespace VoiceInsert.App.Services;

public sealed class SecretStore(AppPaths paths)
{
    public string LoadApiKey()
    {
        return LoadApiKey("default");
    }

    public string LoadApiKey(string profileId)
    {
        if (!File.Exists(paths.SecretPath))
        {
            return "";
        }

        try
        {
            var keys = LoadApiKeys();
            return keys.TryGetValue(profileId, out var apiKey)
                ? apiKey
                : keys.TryGetValue("default", out var defaultKey) ? defaultKey : "";
        }
        catch (CryptographicException)
        {
            File.Delete(paths.SecretPath);
            return "";
        }
        catch (IOException)
        {
            return "";
        }
    }

    public void SaveApiKey(string apiKey)
    {
        SaveApiKey("default", apiKey);
    }

    public void SaveApiKey(string profileId, string apiKey)
    {
        Directory.CreateDirectory(paths.AppDataDirectory);
        var keys = LoadApiKeys().ToDictionary();

        if (string.IsNullOrWhiteSpace(apiKey))
        {
            keys.Remove(profileId);
        }
        else
        {
            keys[profileId] = apiKey;
        }

        SaveApiKeys(keys);
    }

    public IReadOnlyDictionary<string, string> LoadApiKeys()
    {
        if (!File.Exists(paths.SecretPath))
        {
            return new Dictionary<string, string>();
        }

        try
        {
            var encrypted = File.ReadAllBytes(paths.SecretPath);
            var plain = ProtectedData.Unprotect(encrypted, null, DataProtectionScope.CurrentUser);
            var text = Encoding.UTF8.GetString(plain);
            if (text.TrimStart().StartsWith('{'))
            {
                return System.Text.Json.JsonSerializer.Deserialize<Dictionary<string, string>>(text)
                    ?? new Dictionary<string, string>();
            }

            return new Dictionary<string, string> { ["default"] = text };
        }
        catch (CryptographicException)
        {
            File.Delete(paths.SecretPath);
            return new Dictionary<string, string>();
        }
        catch (IOException)
        {
            return new Dictionary<string, string>();
        }
    }

    public void SaveApiKeys(IReadOnlyDictionary<string, string> apiKeys)
    {
        Directory.CreateDirectory(paths.AppDataDirectory);

        if (apiKeys.Count == 0)
        {
            if (File.Exists(paths.SecretPath))
            {
                File.Delete(paths.SecretPath);
            }

            return;
        }

        var json = System.Text.Json.JsonSerializer.Serialize(apiKeys);
        var plain = Encoding.UTF8.GetBytes(json);
        var encrypted = ProtectedData.Protect(plain, null, DataProtectionScope.CurrentUser);
        File.WriteAllBytes(paths.SecretPath, encrypted);
    }
}
