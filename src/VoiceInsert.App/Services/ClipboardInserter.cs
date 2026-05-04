using System.Runtime.InteropServices;

namespace VoiceInsert.App.Services;

public sealed class ClipboardInserter(SettingsService settings)
{
    private const uint KeyEventKeyUp = 0x0002;
    private const ushort VkControl = 0x11;
    private const ushort VkV = 0x56;
    private const int InputKeyboard = 1;
    private const int ClipboardRetryCount = 5;

    public async Task InsertAsync(string text, IntPtr targetWindow)
    {
        object? previousClipboard = await WithClipboardRetryAsync(() =>
            System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
            {
                if (settings.Current.RestoreClipboardContent
                    && System.Windows.Clipboard.ContainsData(System.Windows.DataFormats.UnicodeText))
                {
                    return System.Windows.Clipboard.GetData(System.Windows.DataFormats.UnicodeText);
                }

                return null;
            }).Task);

        await WithClipboardRetryAsync(() =>
            System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
            {
                System.Windows.Clipboard.SetText(text);
                return true;
            }).Task);

        await Task.Delay(settings.Current.DelayBeforePasteMilliseconds);

        if (targetWindow != IntPtr.Zero)
        {
            _ = SetForegroundWindow(targetWindow);
            await Task.Delay(40);
        }

        SendCtrlV();

        if (settings.Current.RestoreClipboardContent && previousClipboard is not null)
        {
            await Task.Delay(settings.Current.DelayBeforeClipboardRestoreMilliseconds);
            await WithClipboardRetryAsync(() =>
                System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
                {
                    System.Windows.Clipboard.SetData(System.Windows.DataFormats.UnicodeText, previousClipboard);
                    return true;
                }).Task);
        }
    }

    private static async Task<T> WithClipboardRetryAsync<T>(Func<Task<T>> operation)
    {
        Exception? lastException = null;
        for (var attempt = 0; attempt < ClipboardRetryCount; attempt++)
        {
            try
            {
                return await operation();
            }
            catch (ExternalException exception)
            {
                lastException = exception;
                await Task.Delay(50 * (attempt + 1));
            }
        }

        throw new InvalidOperationException("Clipboard is busy.", lastException);
    }

    private static void SendCtrlV()
    {
        var inputs = new[]
        {
            Input.Keyboard(VkControl, 0),
            Input.Keyboard(VkV, 0),
            Input.Keyboard(VkV, KeyEventKeyUp),
            Input.Keyboard(VkControl, KeyEventKeyUp)
        };

        var sent = SendInput((uint)inputs.Length, inputs, Marshal.SizeOf<Input>());
        if (sent != inputs.Length)
        {
            SendCtrlVWithKeybdEvent();
            return;
        }
    }

    private static void SendCtrlVWithKeybdEvent()
    {
        keybd_event((byte)VkControl, 0, 0, UIntPtr.Zero);
        keybd_event((byte)VkV, 0, 0, UIntPtr.Zero);
        keybd_event((byte)VkV, 0, KeyEventKeyUp, UIntPtr.Zero);
        keybd_event((byte)VkControl, 0, KeyEventKeyUp, UIntPtr.Zero);
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint nInputs, Input[] pInputs, int cbSize);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);

    [StructLayout(LayoutKind.Sequential)]
    private struct Input
    {
        public uint Type;
        public InputUnion Union;

        public static Input Keyboard(ushort virtualKey, uint flags)
        {
            return new Input
            {
                Type = InputKeyboard,
                Union = new InputUnion
                {
                    KeyboardInput = new KeyboardInput
                    {
                        VirtualKey = virtualKey,
                        Flags = flags
                    }
                }
            };
        }
    }

    [StructLayout(LayoutKind.Explicit)]
    private struct InputUnion
    {
        [FieldOffset(0)] public KeyboardInput KeyboardInput;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct KeyboardInput
    {
        public ushort VirtualKey;
        public ushort ScanCode;
        public uint Flags;
        public uint Time;
        public UIntPtr ExtraInfo;
    }
}
