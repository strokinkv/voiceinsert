namespace VoiceInsert.App.Models;

public sealed class LastErrorState
{
    public DateTimeOffset? OccurredAt { get; private set; }
    public string Message { get; private set; } = "";
    public event EventHandler? Changed;

    public void Set(Exception exception)
    {
        OccurredAt = DateTimeOffset.Now;
        Message = exception.Message;
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public void Set(string message)
    {
        OccurredAt = DateTimeOffset.Now;
        Message = message;
        Changed?.Invoke(this, EventArgs.Empty);
    }
}
