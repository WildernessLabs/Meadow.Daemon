using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Meadow;
using Meadow.Avalonia;
using Meadow.Update;
using System.Threading;
using System.Threading.Tasks;

namespace MyAvaloniaApp
{
    public partial class App : AvaloniaMeadowApplication<DesktopLinux>
    {
        private MainWindow? mainWindow;

        public override void Initialize()
        {
            AvaloniaXamlLoader.Load(this);

            LoadMeadowOS();

            base.OnFrameworkInitializationCompleted();
        }

        public override Task MeadowInitialize()
        {
            InvokeOnMainThread((s) =>
            {
                mainWindow?.SetLabel1Text("App Version 2");

                Resolver.Log.LogLevel = Meadow.Logging.LogLevel.Trace;

                if (Resolver.MeadowCloudService != null)
                {
                    Resolver.MeadowCloudService.ConnectionStateChanged += OnCloudConnectionStateChanged;
                    Resolver.UpdateService.UpdateRetrieved += OnUpdateRetrieved;
                    Resolver.UpdateService.UpdateAvailable += OnUpdateAvailable;
                    mainWindow?.SetLabel2Text("App Initialized");
                }
                else
                {
                    mainWindow?.SetLabel2Text("Unable to reach local Meadow.Daemon");
                }
            });

            return Task.CompletedTask;
        }

        private void OnCloudConnectionStateChanged(object? sender, CloudConnectionState e)
        {
            InvokeOnMainThread((s) =>
            {
                mainWindow?.SetLabel2Text($"cloud: {e}");
            });
        }

        private void OnUpdateAvailable(IUpdateService updateService, UpdateInfo info, CancellationTokenSource cancel)
        {
            InvokeOnMainThread((s) =>
            {
                mainWindow?.SetLabel2Text($"update available: {info.Name}");
            });
        }

        private void OnUpdateRetrieved(IUpdateService updateService, UpdateInfo info, CancellationTokenSource cancel)
        {
            InvokeOnMainThread((s) =>
            {
                mainWindow?.SetLabel2Text($"update retrieved: {info.Name}");
            });
        }

        public override void OnFrameworkInitializationCompleted()
        {
            if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
            {
                desktop.MainWindow = mainWindow = new MainWindow();
            }
        }
    }
}