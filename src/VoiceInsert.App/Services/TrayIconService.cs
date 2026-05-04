using System.Drawing;
using System.Windows;
using VoiceInsert.App.Models;
using Forms = System.Windows.Forms;

namespace VoiceInsert.App.Services;

public sealed class TrayIconService : IDisposable
{
    private readonly Forms.NotifyIcon _notifyIcon;
    private readonly Func<Window> _settingsWindowFactory;
    private readonly SettingsService _settings;
    private readonly Forms.ToolStripMenuItem _settingsItem;
    private readonly Forms.ToolStripMenuItem _exitItem;

    public TrayIconService(Func<Window> settingsWindowFactory, SettingsService settings)
    {
        _settingsWindowFactory = settingsWindowFactory;
        _settings = settings;
        _settingsItem = new Forms.ToolStripMenuItem();
        _settingsItem.Click += (_, _) => ShowSettings();
        _exitItem = new Forms.ToolStripMenuItem();
        _exitItem.Click += (_, _) =>
        {
            if (System.Windows.Application.Current is App app)
            {
                app.RequestShutdown();
                return;
            }

            System.Windows.Application.Current.Shutdown();
        };
        _notifyIcon = new Forms.NotifyIcon
        {
            Icon = LoadIcon(),
            Text = "VoiceInsert",
            Visible = true,
            ContextMenuStrip = BuildMenu()
        };
        _notifyIcon.DoubleClick += (_, _) => ShowSettings();
    }

    private Forms.ContextMenuStrip BuildMenu()
    {
        var menu = new Forms.ContextMenuStrip();
        menu.Opening += (_, _) => ApplyLanguage();
        menu.Items.Add(_settingsItem);
        menu.Items.Add(_exitItem);
        ApplyLanguage();
        return menu;
    }

    private void ApplyLanguage()
    {
        var texts = SettingsTexts.For(_settings.Current.UiLanguage);
        _settingsItem.Text = texts.TraySettings;
        _exitItem.Text = texts.TrayExit;
    }

    private void ShowSettings()
    {
        var window = _settingsWindowFactory();
        window.Show();
        window.Activate();
    }

    public void Dispose()
    {
        _notifyIcon.Visible = false;
        _notifyIcon.Dispose();
    }

    private static Icon LoadIcon()
    {
        var resource = System.Windows.Application.GetResourceStream(
            new Uri("pack://application:,,,/Assets/VoiceInsert.ico"));
        return resource is null ? SystemIcons.Application : new Icon(resource.Stream);
    }
}
