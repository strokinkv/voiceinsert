using VoiceInsert.App.Services;

namespace VoiceInsert.App.Tests;

public sealed class HotkeyMatcherTests
{
    [Theory]
    [InlineData("Ctrl+Space", 0x20)]
    [InlineData("Alt+Y", 0x59)]
    [InlineData("Shift+F12", 0x7B)]
    public void ParseMainKey_ReturnsExpectedVirtualKey(string hotkey, int expected)
    {
        Assert.Equal(expected, HotkeyMatcher.ParseMainKey(hotkey));
    }

    [Fact]
    public void IsModifierRequirementMet_RequiresOnlyConfiguredModifiers()
    {
        Assert.True(HotkeyMatcher.IsModifierRequirementMet("Ctrl+Space", "Ctrl", true));
        Assert.False(HotkeyMatcher.IsModifierRequirementMet("Ctrl+Space", "Ctrl", false));
        Assert.True(HotkeyMatcher.IsModifierRequirementMet("Ctrl+Space", "Alt", false));
    }

    [Theory]
    [InlineData("shift+ctrl+f12", "Ctrl+Shift+F12")]
    [InlineData("alt+y", "Alt+Y")]
    [InlineData("Ctrl+Return", "Ctrl+Enter")]
    public void TryNormalize_NormalizesValidHotkeys(string hotkey, string expected)
    {
        Assert.True(HotkeyMatcher.TryNormalize(hotkey, out var normalized));
        Assert.Equal(expected, normalized);
    }

    [Theory]
    [InlineData("Y")]
    [InlineData("Ctrl")]
    [InlineData("Ctrl+UnknownKey")]
    [InlineData("Meta+Y")]
    public void TryNormalize_RejectsUnsafeOrUnknownHotkeys(string hotkey)
    {
        Assert.False(HotkeyMatcher.TryNormalize(hotkey, out _));
    }
}
