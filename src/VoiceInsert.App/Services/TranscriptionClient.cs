using System.Globalization;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text.Json;
using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

public sealed class TranscriptionClient(SettingsService settings, HttpClient? httpClient = null) : ITranscriptionClient
{
    private readonly HttpClient _httpClient = httpClient ?? new HttpClient();

    public async Task<string> TranscribeAsync(
        byte[] wavBytes,
        AudioRequestKind requestKind,
        CancellationToken cancellationToken)
    {
        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutCts.CancelAfter(TimeSpan.FromSeconds(settings.Current.RequestTimeoutSeconds));
        var apiKey = settings.ApiKey;

        using var form = new MultipartFormDataContent();
        var audioContent = new ByteArrayContent(wavBytes);
        audioContent.Headers.ContentType = new MediaTypeHeaderValue("audio/wav");
        form.Add(audioContent, "file", "recording.wav");
        var model = settings.Current.Model;
        form.Add(new StringContent(model), "model");

        if (requestKind == AudioRequestKind.Transcription && !string.IsNullOrWhiteSpace(settings.Current.Language))
        {
            form.Add(new StringContent(settings.Current.Language), "language");
        }

        var prompt = requestKind == AudioRequestKind.Translation
            ? settings.Current.TranslationPrompt
            : settings.Current.Prompt;
        if (!string.IsNullOrWhiteSpace(prompt))
        {
            form.Add(new StringContent(prompt), "prompt");
        }

        form.Add(
            new StringContent(settings.Current.Temperature.ToString(CultureInfo.InvariantCulture)),
            "temperature");

        var endpointPath = requestKind == AudioRequestKind.Translation
            ? "/v1/audio/translations"
            : "/v1/audio/transcriptions";
        var endpoint = ApiEndpointValidator.CreateEndpointUri(settings.Current.BaseUrl, endpointPath);
        using var request = new HttpRequestMessage(HttpMethod.Post, endpoint)
        {
            Content = form
        };

        if (!string.IsNullOrWhiteSpace(apiKey))
        {
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", apiKey);
        }

        using var response = await _httpClient.SendAsync(request, timeoutCts.Token);
        if (!response.IsSuccessStatusCode)
        {
            var responseBody = await response.Content.ReadAsStringAsync(timeoutCts.Token);
            throw new InvalidOperationException(
                BuildErrorMessage(requestKind, model, response, responseBody));
        }

        await using var stream = await response.Content.ReadAsStreamAsync(timeoutCts.Token);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: timeoutCts.Token);

        if (document.RootElement.TryGetProperty("text", out var textElement))
        {
            return textElement.GetString() ?? "";
        }

        throw new InvalidOperationException("Audio response does not contain a text field.");
    }

    private static string TrimResponse(string responseBody)
    {
        if (string.IsNullOrWhiteSpace(responseBody))
        {
            return "Response body is empty.";
        }

        const int maxLength = 600;
        responseBody = responseBody.ReplaceLineEndings(" ").Trim();
        return responseBody.Length <= maxLength ? responseBody : responseBody[..maxLength] + "...";
    }

    private static string BuildErrorMessage(
        AudioRequestKind requestKind,
        string model,
        HttpResponseMessage response,
        string responseBody)
    {
        var message =
            $"Audio API request failed: {(int)response.StatusCode} {response.ReasonPhrase}. Model: {model}. {TrimResponse(responseBody)}";

        if (requestKind == AudioRequestKind.Translation)
        {
            message += " The selected model may not support audio translation.";
        }

        return message;
    }
}
