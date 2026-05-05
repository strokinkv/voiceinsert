using System.Windows;
using VoiceInsert.App.Models;
using VoiceInsert.App.Services;

namespace VoiceInsert.App;

public partial class App : System.Windows.Application
{
    private Mutex? _appMutex;
    private TrayIconService? _trayIcon;
    private GlobalHotkeyService? _hotkey;
    private MainWindow? _settingsWindow;

    public AppPaths Paths { get; } = new();
    public LastErrorState LastError { get; } = new();
    public SettingsService Settings { get; private set; } = null!;
    public AutostartService Autostart { get; } = new();
    public ModelsClient ModelsClient { get; private set; } = null!;
    public ApiProfileHealthCheckService ApiProfileHealthCheck { get; private set; } = null!;
    public AudioRecorder Recorder { get; private set; } = null!;
    public VoiceInsertController Controller { get; private set; } = null!;
    public SoundService Sounds { get; private set; } = null!;
    public LoggingService Logger { get; private set; } = null!;
    public bool IsShuttingDown { get; private set; }

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        _appMutex = new Mutex(true, "VoiceInsertAppMutex", out var createdNew);
        if (!createdNew)
        {
            Shutdown();
            return;
        }

        Settings = new SettingsService(Paths, new SecretStore(Paths));
        Settings.Load();

        Logger = new LoggingService(Paths, LastError);
        Logger.Configure(Settings.Current.LogLevel);
        ModelsClient = new ModelsClient(Settings);
        ApiProfileHealthCheck = new ApiProfileHealthCheckService(ModelsClient);
        Recorder = new AudioRecorder(Settings);
        var overlay = new RecordingOverlayService(Settings);
        Sounds = new SoundService(Settings);
        Controller = new VoiceInsertController(
            Settings,
            Recorder,
            new TranscriptionClient(Settings),
            new ClipboardInserter(Settings),
            overlay,
            Sounds,
            Logger);

        try
        {
            _hotkey = new GlobalHotkeyService(Settings);
            _hotkey.Pressed += (_, _) =>
                Dispatcher.InvokeAsync(async () => await Controller.HandleHotkeyPressedAsync(AudioRequestKind.Transcription));
            _hotkey.Released += (_, _) =>
                Dispatcher.InvokeAsync(async () => await Controller.HandleHotkeyReleasedAsync());
            _hotkey.TranslationPressed += (_, _) =>
                Dispatcher.InvokeAsync(async () => await Controller.HandleHotkeyPressedAsync(AudioRequestKind.Translation));
            _hotkey.TranslationReleased += (_, _) =>
                Dispatcher.InvokeAsync(async () => await Controller.HandleHotkeyReleasedAsync());
            _hotkey.Register();
        }
        catch (Exception exception)
        {
            Logger.Error(exception, "Failed to register global hotkey.");
            Sounds.PlayError();
        }

        _trayIcon = new TrayIconService(GetSettingsWindow, Settings);

        if (!Settings.Current.LaunchMinimizedToTray || e.Args.Contains("--settings"))
        {
            GetSettingsWindow().Show();
        }
    }

    protected override void OnExit(ExitEventArgs e)
    {
        IsShuttingDown = true;
        _hotkey?.Dispose();
        _trayIcon?.Dispose();
        Recorder?.Dispose();
        _appMutex?.Dispose();
        Serilog.Log.CloseAndFlush();
        base.OnExit(e);
    }

    public void RequestShutdown()
    {
        IsShuttingDown = true;
        Shutdown();
    }

    private Window GetSettingsWindow()
    {
        if (_settingsWindow is null)
        {
            _settingsWindow = new MainWindow();
            _settingsWindow.Closed += (_, _) => _settingsWindow = null;
        }

        return _settingsWindow;
    }
}
