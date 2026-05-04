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
}
