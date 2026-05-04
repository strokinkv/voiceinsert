using Microsoft.Win32;

namespace VoiceInsert.App.Services;

public sealed class AutostartService
{
    private const string RunKeyPath = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string ValueName = "VoiceInsert";

    public bool IsEnabled()
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKeyPath, false);
        return key?.GetValue(ValueName) is string;
    }

    public void SetEnabled(bool enabled)
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKeyPath, true)
            ?? Registry.CurrentUser.CreateSubKey(RunKeyPath, true);

        if (enabled)
        {
            var exePath = Environment.ProcessPath ?? "";
            key.SetValue(ValueName, $"\"{exePath}\" --tray", RegistryValueKind.String);
            return;
        }

        key.DeleteValue(ValueName, false);
    }
}
