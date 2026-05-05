using System.Media;

namespace VoiceInsert.App.Services;

public sealed class SoundService(SettingsService settings) : ISoundService
{
    public void PlayStart() => PlayResource("record-start.wav", SystemSounds.Asterisk);
    public void PlayStop() => PlayResource("record-stop.wav", SystemSounds.Beep);
    public void PlayError() => Play(SystemSounds.Hand);

    private void Play(SystemSound sound)
    {
        if (!settings.Current.EnableSounds)
        {
            return;
        }

        sound.Play();
    }

    private void PlayResource(string fileName, SystemSound fallback)
    {
        if (!settings.Current.EnableSounds)
        {
            return;
        }

        var resource = System.Windows.Application.GetResourceStream(
            new Uri($"pack://application:,,,/Assets/{fileName}"));
        if (resource is null)
        {
            fallback.Play();
            return;
        }

        using var player = new SoundPlayer(resource.Stream);
        player.Play();
    }
}
