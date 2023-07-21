using Meadow.Daemon;
using Meadow.Update;

internal class Program
{
    private static void Main(string[] args)
    {
        var path = AppDomain.CurrentDomain.BaseDirectory;

        Console.WriteLine($"Running from {path}");

        new MyApp().Run();
    }
}

public class MyApp
{
    private UpdateService _updateService;
    private bool _firstConnection = true;

    public string ServiceAddress { get; set; } = "172.26.8.20";
    public int ServicePort { get; set; } = 5000;

    public MyApp()
    {
        _updateService = new UpdateService(ServiceAddress, ServicePort);

        _updateService.OnUpdateAvailable += OnUpdateAvailable;
        _updateService.UpdateChanged += OnUpdateChanged;
        _updateService.OnUpdateRetrieved += OnUpdateRetrieved;
        _updateService.StateChanged += OnServiceStateChanged;

        _updateService.Start();
    }

    private void OnServiceStateChanged(object? sender, UpdateState e)
    {
        Console.WriteLine($"Update service is now: {e}");

        // on first connect, clear all known updates for this sample
        if (_firstConnection)
        {
            Console.WriteLine($"Clearing update store...");

            _firstConnection = false;
            _updateService.ClearUpdates();
        }

    }

    private void OnUpdateRetrieved(object? sender, UpdateInfo e)
    {
        Console.WriteLine($"An update has been retrieved! (ID: {e.ID})");
    }

    private void OnUpdateChanged(object? sender, UpdateInfo e)
    {
        Console.WriteLine($"An update has changed! (ID: {e.ID})");
    }

    private void OnUpdateAvailable(object? sender, UpdateInfo e)
    {
        Console.WriteLine($"An update is available! (ID: {e.ID})");
    }

    public void Run()
    {
        while (true)
        {
            // this is just your app doing "stuff"
            Thread.Sleep(1000);
        }
    }
}
