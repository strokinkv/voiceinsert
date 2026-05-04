using VoiceInsert.App.Models;
using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class RecordingModePolicyTests
{
    [Theory]
    [InlineData(RecordingMode.Toggle, false)]
    [InlineData(RecordingMode.Hold, false)]
    [InlineData(RecordingMode.SilenceTimeout, true)]
    public void StopsOnSilence_OnlyForSilenceTimeout(RecordingMode mode, bool expected)
    {
        Assert.Equal(expected, RecordingModePolicy.StopsOnSilence(mode));
    }
}
