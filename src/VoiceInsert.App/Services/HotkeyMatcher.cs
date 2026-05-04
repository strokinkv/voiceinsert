namespace VoiceInsert.App.Services;

internal static class HotkeyMatcher
{
    public const int VkSpace = 0x20;

    public static int ParseMainKey(string hotkey)
    {
        var key = hotkey.Split('+', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries)
            .LastOrDefault() ?? "Space";
        return key.Equals("Space", StringComparison.OrdinalIgnoreCase) ? VkSpace : KeyNameToVirtualKey(key);
    }

    public static bool IsModifierRequirementMet(string hotkey, string modifier, bool isPressed)
    {
        return !hotkey.Split('+', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries)
            .Contains(modifier, StringComparer.OrdinalIgnoreCase) || isPressed;
    }

    public static int KeyNameToVirtualKey(string key)
    {
        if (key.Length == 1)
        {
            return char.ToUpperInvariant(key[0]);
        }

        return key.ToUpperInvariant() switch
        {
            "ENTER" or "RETURN" => 0x0D,
            "ESC" or "ESCAPE" => 0x1B,
            "TAB" => 0x09,
            "BACK" or "BACKSPACE" => 0x08,
            "DELETE" or "DEL" => 0x2E,
            "INSERT" or "INS" => 0x2D,
            "UP" => 0x26,
            "DOWN" => 0x28,
            "LEFT" => 0x25,
            "RIGHT" => 0x27,
            "HOME" => 0x24,
            "END" => 0x23,
            "PAGEUP" => 0x21,
            "PAGEDOWN" => 0x22,
            _ when key.StartsWith('F') && int.TryParse(key[1..], out var number) && number is >= 1 and <= 24 => 0x70 + number - 1,
            _ => VkSpace
        };
    }
}
