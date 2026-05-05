namespace VoiceInsert.App.Services;

internal static class HotkeyMatcher
{
    public const int VkSpace = 0x20;
    private static readonly string[] ModifierOrder = ["Ctrl", "Alt", "Shift", "Win"];

    public static int ParseMainKey(string hotkey)
    {
        var key = SplitHotkey(hotkey).LastOrDefault() ?? "Space";
        return TryGetVirtualKey(key, out var virtualKey) ? virtualKey : VkSpace;
    }

    public static bool IsModifierRequirementMet(string hotkey, string modifier, bool isPressed)
    {
        return !SplitHotkey(hotkey).Contains(modifier, StringComparer.OrdinalIgnoreCase) || isPressed;
    }

    public static bool TryNormalize(string hotkey, out string normalized)
    {
        normalized = "";
        var parts = SplitHotkey(hotkey);
        if (parts.Length < 2)
        {
            return false;
        }

        var key = parts[^1];
        if (!TryGetVirtualKey(key, out _))
        {
            return false;
        }

        var modifiers = parts[..^1];
        if (modifiers.Length == 0
            || modifiers.Any(modifier => !ModifierOrder.Contains(modifier, StringComparer.OrdinalIgnoreCase)))
        {
            return false;
        }

        var normalizedParts = ModifierOrder
            .Where(modifier => modifiers.Contains(modifier, StringComparer.OrdinalIgnoreCase))
            .ToList();
        normalizedParts.Add(NormalizeKeyName(key));
        normalized = string.Join("+", normalizedParts);
        return true;
    }

    public static int KeyNameToVirtualKey(string key)
    {
        return TryGetVirtualKey(key, out var virtualKey) ? virtualKey : VkSpace;
    }

    private static string[] SplitHotkey(string hotkey)
    {
        return hotkey.Split('+', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
    }

    private static bool TryGetVirtualKey(string key, out int virtualKey)
    {
        virtualKey = 0;
        if (key.Length == 1 && char.IsLetterOrDigit(key[0]))
        {
            virtualKey = char.ToUpperInvariant(key[0]);
            return true;
        }

        var upperKey = key.ToUpperInvariant();
        virtualKey = upperKey switch
        {
            "SPACE" => VkSpace,
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
            _ when upperKey.StartsWith('F') && int.TryParse(upperKey[1..], out var number) && number is >= 1 and <= 24 => 0x70 + number - 1,
            _ => 0
        };

        return virtualKey != 0;
    }

    private static string NormalizeKeyName(string key)
    {
        if (key.Length == 1 && char.IsLetterOrDigit(key[0]))
        {
            return char.ToUpperInvariant(key[0]).ToString();
        }

        var upperKey = key.ToUpperInvariant();
        return upperKey switch
        {
            "RETURN" => "Enter",
            "ESCAPE" => "Esc",
            "BACK" => "Backspace",
            "DEL" => "Delete",
            "INS" => "Insert",
            "PAGEUP" => "PageUp",
            "PAGEDOWN" => "PageDown",
            _ when upperKey.StartsWith('F') && int.TryParse(upperKey[1..], out var number) && number is >= 1 and <= 24 => $"F{number}",
            _ => key[..1].ToUpperInvariant() + key[1..].ToLowerInvariant()
        };
    }
}
