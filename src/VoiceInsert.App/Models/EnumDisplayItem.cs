using CommunityToolkit.Mvvm.ComponentModel;

namespace VoiceInsert.App.Models;

public sealed partial class EnumDisplayItem<T>(T value, string displayName) : ObservableObject
    where T : struct, Enum
{
    public T Value { get; } = value;

    [ObservableProperty]
    private string _displayName = displayName;
}
