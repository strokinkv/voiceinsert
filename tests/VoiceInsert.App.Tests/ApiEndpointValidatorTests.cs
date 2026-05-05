using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class ApiEndpointValidatorTests
{
    [Theory]
    [InlineData("http://localhost:9573", "v1/models", "http://localhost:9573/v1/models")]
    [InlineData("http://localhost:9573/", "/v1/audio/transcriptions", "http://localhost:9573/v1/audio/transcriptions")]
    [InlineData("https://api.example.com/openai/", "v1/models", "https://api.example.com/openai/v1/models")]
    public void CreateEndpointUri_ReturnsExpectedUri(string baseUrl, string endpointPath, string expected)
    {
        var uri = ApiEndpointValidator.CreateEndpointUri(baseUrl, endpointPath);

        Assert.Equal(expected, uri.ToString());
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData("file:///tmp/api")]
    [InlineData("not a url")]
    public void CreateEndpointUri_ThrowsHelpfulErrorForInvalidBaseUrl(string baseUrl)
    {
        var exception = Assert.Throws<InvalidOperationException>(() =>
            ApiEndpointValidator.CreateEndpointUri(baseUrl, "v1/models"));

        Assert.Contains("API base URL", exception.Message);
    }
}
