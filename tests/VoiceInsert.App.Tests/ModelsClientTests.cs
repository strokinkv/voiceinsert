using System.Net;
using VoiceInsert.App.Models;
using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class ModelsClientTests
{
    [Fact]
    public async Task GetModelsAsync_UsesExplicitProfileValues()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            BaseUrl = "http://wlast.local",
            Model = "wlast-model"
        });
        using var handler = new CapturingHandler();
        var client = new ModelsClient(fixture.Settings, new HttpClient(handler));

        var models = await client.GetModelsAsync(
            "http://groq.local",
            "groq-key",
            TestContext.Current.CancellationToken);

        Assert.Single(models);
        Assert.Equal("groq-model", models[0].Id);
        Assert.Equal("http://groq.local/v1/models", handler.RequestUri?.ToString());
        Assert.Equal("Bearer", handler.AuthorizationScheme);
        Assert.Equal("groq-key", handler.AuthorizationParameter);
    }

    private sealed class CapturingHandler : HttpMessageHandler
    {
        public Uri? RequestUri { get; private set; }
        public string? AuthorizationScheme { get; private set; }
        public string? AuthorizationParameter { get; private set; }

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            RequestUri = request.RequestUri;
            AuthorizationScheme = request.Headers.Authorization?.Scheme;
            AuthorizationParameter = request.Headers.Authorization?.Parameter;
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new StringContent("""{"data":[{"id":"groq-model"}]}""")
            });
        }
    }

    private sealed class SettingsFixture : IDisposable
    {
        private readonly string _root = Path.Combine(Path.GetTempPath(), "VoiceInsertModelTests", Guid.NewGuid().ToString("N"));

        public SettingsFixture(AppSettings appSettings)
        {
            var paths = new AppPaths(Path.Combine(_root, "app"), Path.Combine(_root, "local"));
            Settings = new SettingsService(paths, new SecretStore(paths));
            Settings.Save(appSettings, "");
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
