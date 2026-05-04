using System.Diagnostics;
using System.Runtime.InteropServices;

namespace VoiceInsert.App.Services;

public sealed class GlobalHotkeyService : IDisposable
{
    private const int WhKeyboardLl = 13;
    private const int WmKeyDown = 0x0100;
    private const int WmSysKeyDown = 0x0104;
    private const int WmKeyUp = 0x0101;
    private const int WmSysKeyUp = 0x0105;
    private const int VkControl = 0x11;
    private const int VkLControl = 0xA2;
    private const int VkRControl = 0xA3;
    private readonly LowLevelKeyboardProc _proc;
    private readonly SettingsService _settings;
    private IntPtr _hookId;
    private bool _isHotkeyDown;
    private bool _isTranslationHotkeyDown;

    public GlobalHotkeyService(SettingsService settings)
    {
        _settings = settings;
        _proc = HookCallback;
    }

    public event EventHandler? Pressed;
    public event EventHandler? Released;
    public event EventHandler? TranslationPressed;
    public event EventHandler? TranslationReleased;

    public void Register()
    {
        using var currentProcess = Process.GetCurrentProcess();
        using var currentModule = currentProcess.MainModule;
        var moduleHandle = currentModule?.ModuleName is null ? IntPtr.Zero : GetModuleHandle(currentModule.ModuleName);
        _hookId = SetWindowsHookEx(WhKeyboardLl, _proc, moduleHandle, 0);

        if (_hookId == IntPtr.Zero)
        {
            throw new InvalidOperationException("Failed to install global keyboard hook.");
        }
    }

    private IntPtr HookCallback(int nCode, IntPtr wParam, IntPtr lParam)
    {
        if (nCode >= 0)
        {
            var message = wParam.ToInt32();
            var virtualKey = Marshal.ReadInt32(lParam);

            if (IsKeyDownMessage(message)
                && MatchesConfiguredHotkey(virtualKey, _settings.Current.TranslationHotkey))
            {
                if (!_isTranslationHotkeyDown)
                {
                    _isTranslationHotkeyDown = true;
                    TranslationPressed?.Invoke(this, EventArgs.Empty);
                }
            }
            else if (IsKeyDownMessage(message)
                && MatchesConfiguredHotkey(virtualKey, _settings.Current.Hotkey))
            {
                if (!_isHotkeyDown)
                {
                    _isHotkeyDown = true;
                    Pressed?.Invoke(this, EventArgs.Empty);
                }
            }

            if (IsKeyUpMessage(message) && ShouldRelease(virtualKey, _settings.Current.Hotkey, _isHotkeyDown))
            {
                _isHotkeyDown = false;
                Released?.Invoke(this, EventArgs.Empty);
            }

            if (IsKeyUpMessage(message)
                && ShouldRelease(virtualKey, _settings.Current.TranslationHotkey, _isTranslationHotkeyDown))
            {
                _isTranslationHotkeyDown = false;
                TranslationReleased?.Invoke(this, EventArgs.Empty);
            }
        }

        return CallNextHookEx(_hookId, nCode, wParam, lParam);
    }

    private static bool ShouldRelease(int virtualKey, string hotkey, bool isDown)
    {
        return isDown && (virtualKey == HotkeyMatcher.ParseMainKey(hotkey) || IsModifierKey(virtualKey));
    }

    private static bool MatchesConfiguredHotkey(int virtualKey, string hotkey)
    {
        return virtualKey == HotkeyMatcher.ParseMainKey(hotkey)
            && HotkeyMatcher.IsModifierRequirementMet(hotkey, "Ctrl", IsControlPressed())
            && HotkeyMatcher.IsModifierRequirementMet(hotkey, "Alt", IsAltPressed())
            && HotkeyMatcher.IsModifierRequirementMet(hotkey, "Shift", IsShiftPressed())
            && HotkeyMatcher.IsModifierRequirementMet(hotkey, "Win", IsWinPressed());
    }

    private static bool IsControlPressed()
    {
        return (GetAsyncKeyState(VkControl) & 0x8000) != 0
            || (GetAsyncKeyState(VkLControl) & 0x8000) != 0
            || (GetAsyncKeyState(VkRControl) & 0x8000) != 0;
    }

    private static bool IsAltPressed() => (GetAsyncKeyState(0x12) & 0x8000) != 0;
    private static bool IsShiftPressed() => (GetAsyncKeyState(0x10) & 0x8000) != 0;
    private static bool IsWinPressed() => (GetAsyncKeyState(0x5B) & 0x8000) != 0 || (GetAsyncKeyState(0x5C) & 0x8000) != 0;

    private static bool IsKeyDownMessage(int message) => message is WmKeyDown or WmSysKeyDown;
    private static bool IsKeyUpMessage(int message) => message is WmKeyUp or WmSysKeyUp;
    private static bool IsModifierKey(int virtualKey) => virtualKey is VkControl or VkLControl or VkRControl or 0x12 or 0x10 or 0x5B or 0x5C;

    public void Dispose()
    {
        if (_hookId != IntPtr.Zero)
        {
            UnhookWindowsHookEx(_hookId);
            _hookId = IntPtr.Zero;
        }
    }

    private delegate IntPtr LowLevelKeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetWindowsHookEx(int idHook, LowLevelKeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool UnhookWindowsHookEx(IntPtr hhk);

    [DllImport("user32.dll")]
    private static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);

    [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    private static extern IntPtr GetModuleHandle(string lpModuleName);

    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int vKey);
}
