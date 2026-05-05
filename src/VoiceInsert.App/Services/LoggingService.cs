using Serilog;
using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

public sealed class LoggingService(AppPaths paths, LastErrorState lastError) : ILoggingService
{
    public void Configure(string minimumLevel = "Information")
    {
        Directory.CreateDirectory(paths.LogsDirectory);
        var level = minimumLevel?.Trim().ToLowerInvariant() switch
        {
            "debug" => Serilog.Events.LogEventLevel.Debug,
            "warning" => Serilog.Events.LogEventLevel.Warning,
            "error" => Serilog.Events.LogEventLevel.Error,
            _ => Serilog.Events.LogEventLevel.Information
        };

        Log.Logger = new LoggerConfiguration()
            .MinimumLevel.Is(level)
            .WriteTo.File(
                Path.Combine(paths.LogsDirectory, "voiceinsert-.log"),
                rollingInterval: RollingInterval.Day,
                retainedFileCountLimit: 14,
                outputTemplate: "{Timestamp:yyyy-MM-dd HH:mm:ss.fff zzz} [{Level:u3}] {Message:lj}{NewLine}{Exception}")
            .CreateLogger();
    }

    public void Error(Exception exception, string safeMessage)
    {
        lastError.Set(safeMessage);
        Log.Error(exception, "{SafeMessage}", safeMessage);
    }

    public void Error(string safeMessage)
    {
        lastError.Set(safeMessage);
        Log.Error("{SafeMessage}", safeMessage);
    }

    public void Information(string message)
    {
        Log.Information("{Message}", message);
    }
}
