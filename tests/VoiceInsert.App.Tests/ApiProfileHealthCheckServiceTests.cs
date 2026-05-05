using System.Net;
using VoiceInsert.App.Models;
using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class ApiProfileHealthCheckServiceTests
{
    [Fact]
    public async Task CheckAsync_ReturnsHealthyWhenSelectedModelExists()
    {
        using var fixture = new SettingsFixture();
        using var handler = new StaticJsonHandler("""{"data":[{"id":"whisper-large-v3"}]}""");
        var service = new ApiProfileHealthCheckService(new ModelsClient(fixture.Settings, new HttpClient(handler)));

        var result = await service.CheckAsync(
            new ApiProfile
            {
                BaseUrl = "http://localhost:9573",
                Model = "whisper-large-v3"
            },
            "api-key",
            TestContext.Current.CancellationToken);

        Assert.True(result.IsHealthy);
        Assert.Equal(1, result.ModelCount);
        Assert.True(result.SelectedModelFound);
    }

    [Fact]
    public async Task CheckAsync_ReturnsUnhealthyWhenSelectedModelIsMissing()
    {
        using var fixture = new SettingsFixture();
        using var handler = new StaticJsonHandler("""{"data":[{"id":"other-model"}]}""");
        var service = new ApiProfileHealthCheckService(new ModelsClient(fixture.Settings, new HttpClient(handler)));

        var result = await service.CheckAsync(
            new ApiProfile
            {
                BaseUrl = "http://localhost:9573",
                Model = "whisper-large-v3"
            },
            "api-key",
            TestContext.Current.CancellationToken);

        Assert.False(result.IsHealthy);
        Assert.Equal(1, result.ModelCount);
        Assert.False(result.SelectedModelFound);
        Assert.Contains("selected model", result.Message);
    }

    [Fact]
    public async Task CheckAsync_ReturnsValidationErrorForInvalidUrl()
    {
        using var fixture = new SettingsFixture();
        using var handler = new StaticJsonHandler("""{"data":[]}""");
        var service = new ApiProfileHealthCheckService(new ModelsClient(fixture.Settings, new HttpClient(handler)));

        var result = await service.CheckAsync(
            new ApiProfile
            {
                BaseUrl = "file:///tmp/api",
                Model = "whisper-large-v3"
            },
            "api-key",
            TestContext.Current.CancellationToken);

        Assert.False(result.IsHealthy);
        Assert.Contains("http or https", result.Message);
    }

    private sealed class StaticJsonHandler(string responseBody) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new StringContent(responseBody)
            });
        }
    }

    private sealed class SettingsFixture : IDisposable
    {
        private readonly string _root = Path.Combine(Path.GetTempPath(), "VoiceInsertHealthCheckTests", Guid.NewGuid().ToString("N"));

        public SettingsFixture()
        {
            var paths = new AppPaths(Path.Combine(_root, "app"), Path.Combine(_root, "local"));
            Settings = new SettingsService(paths, new SecretStore(paths));
            Settings.Save(new AppSettings(), "");
        }

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
