using System.Net.Http;
using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

public sealed record ApiProfileHealthCheckResult(
    bool IsHealthy,
    string Message,
    int ModelCount,
    bool SelectedModelFound);

public sealed class ApiProfileHealthCheckService(ModelsClient modelsClient)
{
    public async Task<ApiProfileHealthCheckResult> CheckAsync(
        ApiProfile profile,
        string apiKey,
        CancellationToken cancellationToken)
    {
        if (!ApiEndpointValidator.TryCreateBaseUri(profile.BaseUrl, out _, out var validationError))
        {
            return new ApiProfileHealthCheckResult(false, validationError, 0, false);
        }

        try
        {
            var models = await modelsClient.GetModelsAsync(profile.BaseUrl, apiKey, cancellationToken);
            var selectedModelFound = string.IsNullOrWhiteSpace(profile.Model)
                || models.Any(model => model.Id.Equals(profile.Model, StringComparison.OrdinalIgnoreCase));

            if (models.Count == 0)
            {
                return new ApiProfileHealthCheckResult(
                    false,
                    "API profile is reachable, but /v1/models returned no models.",
                    0,
                    selectedModelFound);
            }

            if (!selectedModelFound)
            {
                return new ApiProfileHealthCheckResult(
                    false,
                    $"API profile is reachable, but selected model '{profile.Model}' was not returned by /v1/models.",
                    models.Count,
                    false);
            }

            return new ApiProfileHealthCheckResult(
                true,
                $"API profile is reachable. Models returned: {models.Count}.",
                models.Count,
                true);
        }
        catch (HttpRequestException exception)
        {
            return new ApiProfileHealthCheckResult(false, $"API profile request failed: {exception.Message}", 0, false);
        }
        catch (TaskCanceledException exception) when (!cancellationToken.IsCancellationRequested)
        {
            return new ApiProfileHealthCheckResult(false, $"API profile request timed out: {exception.Message}", 0, false);
        }
        catch (InvalidOperationException exception)
        {
            return new ApiProfileHealthCheckResult(false, exception.Message, 0, false);
        }
        catch (System.Text.Json.JsonException exception)
        {
            return new ApiProfileHealthCheckResult(false, $"API profile returned invalid JSON: {exception.Message}", 0, false);
        }
    }
}
