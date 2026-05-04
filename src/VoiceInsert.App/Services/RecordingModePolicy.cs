using VoiceInsert.App.Models;

namespace VoiceInsert.App.Services;

internal static class RecordingModePolicy
{
    public static bool StopsOnSilence(RecordingMode mode) => mode == RecordingMode.SilenceTimeout;
}
