namespace VoiceInsert.App.Services;

internal static class ApiEndpointValidator
{
    public static Uri CreateEndpointUri(string baseUrl, string endpointPath)
    {
        if (!TryCreateBaseUri(baseUrl, out var baseUri, out var errorMessage))
        {
            throw new InvalidOperationException(errorMessage);
        }

        return new Uri(baseUri, endpointPath.TrimStart('/'));
    }

    public static bool TryCreateBaseUri(string baseUrl, out Uri baseUri, out string errorMessage)
    {
        baseUri = null!;

        if (string.IsNullOrWhiteSpace(baseUrl))
        {
            errorMessage = "API base URL is empty.";
            return false;
        }

        var normalized = baseUrl.Trim();
        if (!Uri.TryCreate(normalized.TrimEnd('/') + "/", UriKind.Absolute, out var parsedBaseUri))
        {
            errorMessage = $"API base URL is invalid: {normalized}";
            return false;
        }

        baseUri = parsedBaseUri;
        if (baseUri.Scheme is not "http" and not "https")
        {
            errorMessage = $"API base URL must use http or https scheme: {normalized}";
            return false;
        }

        errorMessage = "";
        return true;
    }
}
