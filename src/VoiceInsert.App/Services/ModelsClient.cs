using System.Net.Http;
using System.Text.Json;
using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

public sealed class ModelsClient(SettingsService settings, HttpClient? httpClient = null)
{
    private readonly HttpClient _httpClient = httpClient ?? new HttpClient();

    public async Task<IReadOnlyList<TranscriptionModel>> GetModelsAsync(CancellationToken cancellationToken)
    {
        return await GetModelsAsync(settings.Current.BaseUrl, settings.ApiKey, cancellationToken);
    }

    public async Task<IReadOnlyList<TranscriptionModel>> GetModelsAsync(
        string baseUrl,
        string apiKey,
        CancellationToken cancellationToken)
    {
        var endpoint = ApiEndpointValidator.CreateEndpointUri(baseUrl, "v1/models");
        using var request = new HttpRequestMessage(HttpMethod.Get, endpoint);
        if (!string.IsNullOrWhiteSpace(apiKey))
        {
            request.Headers.Authorization =
                new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", apiKey);
        }

        using var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);
        var root = document.RootElement;

        if (root.ValueKind == JsonValueKind.Array)
        {
            return root.EnumerateArray()
                .Select(ReadModel)
                .Where(static model => !string.IsNullOrWhiteSpace(model.Id))
                .ToArray();
        }

        if (root.TryGetProperty("data", out var data) && data.ValueKind == JsonValueKind.Array)
        {
            return data.EnumerateArray()
                .Select(ReadModel)
                .Where(static model => !string.IsNullOrWhiteSpace(model.Id))
                .ToArray();
        }

        return [];
    }

    private static TranscriptionModel ReadModel(JsonElement element)
    {
        return element.ValueKind switch
        {
            JsonValueKind.String => new TranscriptionModel(element.GetString() ?? ""),
            JsonValueKind.Object when element.TryGetProperty("id", out var id) => new TranscriptionModel(id.GetString() ?? ""),
            _ => new TranscriptionModel("")
        };
    }
}
