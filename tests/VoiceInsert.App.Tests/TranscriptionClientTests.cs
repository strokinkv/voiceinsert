using System.Net;
using VoiceInsert.App.Models;
using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class TranscriptionClientTests
{
    [Fact]
    public async Task TranscribeAsync_UsesTranscriptionEndpointAndLanguage()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            BaseUrl = "http://localhost:9573",
            Model = "whisper-test",
            Language = "ru",
            Prompt = "domain prompt"
        });
        using var handler = new CapturingHandler();
        var client = new TranscriptionClient(fixture.Settings, new HttpClient(handler));

        var text = await client.TranscribeAsync(
            [1, 2, 3],
            AudioRequestKind.Transcription,
            TestContext.Current.CancellationToken);

        Assert.Equal("ok", text);
        Assert.Equal("http://localhost:9573/v1/audio/transcriptions", handler.RequestUri?.ToString());
        var body = await handler.Body!.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("name=language", body);
        Assert.Contains("ru", body);
        Assert.Contains("domain prompt", body);
    }

    [Fact]
    public async Task TranscribeAsync_UsesTranslationEndpointAndSameModelWithoutLanguage()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            BaseUrl = "http://localhost:9573",
            Model = "whisper-test",
            TranslationModel = "whisper-translation",
            Language = "ru",
            Prompt = "transcription prompt",
            TranslationPrompt = "translation prompt"
        });
        using var handler = new CapturingHandler();
        var client = new TranscriptionClient(fixture.Settings, new HttpClient(handler));

        var text = await client.TranscribeAsync(
            [1, 2, 3],
            AudioRequestKind.Translation,
            TestContext.Current.CancellationToken);

        Assert.Equal("ok", text);
        Assert.Equal("http://localhost:9573/v1/audio/translations", handler.RequestUri?.ToString());
        var body = await handler.Body!.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("whisper-test", body);
        Assert.DoesNotContain("whisper-translation", body);
        Assert.DoesNotContain("name=language", body);
        Assert.DoesNotContain("transcription prompt", body);
        Assert.Contains("translation prompt", body);
    }

    [Fact]
    public async Task TranscribeAsync_ThrowsApiResponseBodyOnError()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            BaseUrl = "http://localhost:9573",
            Model = "whisper-test"
        });
        using var handler = new CapturingHandler
        {
            StatusCode = HttpStatusCode.BadRequest,
            ResponseBody = """{"error":{"message":"translation model is not available"}}"""
        };
        var client = new TranscriptionClient(fixture.Settings, new HttpClient(handler));

        var exception = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            client.TranscribeAsync(
                [1, 2, 3],
                AudioRequestKind.Translation,
                TestContext.Current.CancellationToken));

        Assert.Contains("400 Bad Request", exception.Message);
        Assert.Contains("whisper-test", exception.Message);
        Assert.Contains("translation model is not available", exception.Message);
        Assert.Contains("may not support audio translation", exception.Message);
    }

    [Fact]
    public async Task TranscribeAsync_CanReuseHttpClientAcrossRequests()
    {
        using var fixture = new SettingsFixture(new AppSettings
        {
            BaseUrl = "http://localhost:9573",
            Model = "whisper-test"
        });
        using var handler = new CapturingHandler();
        var client = new TranscriptionClient(fixture.Settings, new HttpClient(handler));

        var first = await client.TranscribeAsync(
            [1, 2, 3],
            AudioRequestKind.Transcription,
            TestContext.Current.CancellationToken);
        var second = await client.TranscribeAsync(
            [4, 5, 6],
            AudioRequestKind.Transcription,
            TestContext.Current.CancellationToken);

        Assert.Equal("ok", first);
        Assert.Equal("ok", second);
        Assert.Equal(2, handler.RequestCount);
    }

    private sealed class CapturingHandler : HttpMessageHandler
    {
        public Uri? RequestUri { get; private set; }
        public HttpContent? Body { get; private set; }
        public int RequestCount { get; private set; }
        public HttpStatusCode StatusCode { get; init; } = HttpStatusCode.OK;
        public string ResponseBody { get; init; } = """{"text":"ok"}""";

        protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            RequestCount++;
            RequestUri = request.RequestUri;
            Body = new StringContent(await request.Content!.ReadAsStringAsync(cancellationToken));
            return new HttpResponseMessage(StatusCode)
            {
                Content = new StringContent(ResponseBody)
            };
        }
    }

    private sealed class SettingsFixture : IDisposable
    {
        private readonly string _root = Path.Combine(Path.GetTempPath(), "VoiceInsertTests", Guid.NewGuid().ToString("N"));

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
