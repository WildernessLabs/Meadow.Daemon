using Meadow.Update;
using System.Diagnostics;
using System.Text.Json;

namespace Meadow.Daemon;

public partial class UpdateService : IUpdateService, IDisposable
{
    public event UpdateEventHandler OnUpdateAvailable = delegate { };
    public event UpdateEventHandler OnUpdateRetrieved = delegate { };
    public event UpdateEventHandler OnUpdateSuccess = delegate { };
    public event UpdateEventHandler OnUpdateFailure = delegate { };

    public bool CanUpdate => State == UpdateState.Idle;
    public UpdateState State { get; private set; }

    public void ClearUpdates() { throw new NotImplementedException(); }

    public event EventHandler Connected = delegate { };
    public event EventHandler Disconnected = delegate { };
    public event EventHandler<UpdateInfo> UpdateChanged = delegate { };

    private Task? _stateMonitor;
    private CancellationTokenSource? _cancellationToken;
    private bool _isDisposed;
    private HttpClient _httpClient;
    private JsonSerializerOptions _serializerOptions;

    protected virtual TimeSpan ServiceCheckPeriod { get; } = TimeSpan.FromSeconds(5);
    protected virtual string ApiRoot { get; } = "/api";

    public DeviceInfo? DeviceInfo { get; private set; }
    public UpdateCollection Updates { get; }

    public UpdateService(string serviceAddress = "127.0.0.1", int servicePort = 5000)
    {
        State = UpdateState.Disconnected;

        Updates = new UpdateCollection();

        _serializerOptions = new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
        };

        _httpClient = new HttpClient();

        serviceAddress.TrimEnd().TrimEnd('/'); // normalize

        if (!serviceAddress.StartsWith("http", StringComparison.OrdinalIgnoreCase))
        {
            serviceAddress = "http://" + serviceAddress;
        }

        serviceAddress = $"{serviceAddress}:{servicePort}";

        _httpClient.BaseAddress = new Uri(serviceAddress);
    }

    public void Start()
    {
        if (_stateMonitor == null)
        {
            _cancellationToken = new CancellationTokenSource();
            _stateMonitor = new Task(() => _ = StateMonitorProc(), _cancellationToken.Token, TaskCreationOptions.LongRunning);
            _stateMonitor.Start();
        }
    }

    public void Stop()
    {
        _cancellationToken?.Cancel();
        _stateMonitor?.Wait();
    }

    public async void RetrieveUpdate(UpdateInfo updateInfo)
    {
        try
        {
            var existing = Updates[updateInfo.ID];

            var payload = new JsonContent(new UpdateAction
            {
                Action = UpdateActions.Download
            });

            var response = await _httpClient.PutAsync(
                $"{ApiRoot}/{(Endpoints.UpdateAction.Replace("{id}", updateInfo.ID))}",
                payload);

            if (!response.IsSuccessStatusCode)
            {
                // TODO: throw an appropriate exception
            }
        }
        catch (Exception ex)
        {
            // TODO: catch only timeout here

            // disconnect
            State = UpdateState.Disconnected;
        }
    }

    public async void ApplyUpdate(UpdateInfo updateInfo)
    {
        try
        {
            var existing = Updates[updateInfo.ID];

            var payload = new JsonContent(new UpdateAction
            {
                Action = UpdateActions.Apply,
                Pid = Process.GetCurrentProcess().Id
            });

            var response = await _httpClient.PutAsync(
                $"{ApiRoot}/{(Endpoints.UpdateAction.Replace("{id}", updateInfo.ID))}",
                payload);

            if (!response.IsSuccessStatusCode)
            {
                // TODO: throw an appropriate exception
            }
        }
        catch (Exception ex)
        {
            // TODO: catch only timeout here

            // disconnect
            State = UpdateState.Disconnected;
        }
    }

    private async Task<DeviceInfo?> GetDeviceInfo()
    {
        try
        {
            var response = await _httpClient.GetAsync($"{ApiRoot}/{Endpoints.DeviceInfo}");
            if (response.IsSuccessStatusCode)
            {
                var info = JsonSerializer.Deserialize<DeviceInfo>(
                    await response.Content.ReadAsStringAsync(),
                    _serializerOptions);

                DeviceInfo = info;
                // TODO: parse out state
                this.State = UpdateState.Connected;
                return info;
            }
        }
        catch (Exception ex)
        {
            this.State = UpdateState.Disconnected;

            // disconnect
            State = UpdateState.Disconnected;
        }

        return null;
    }

    private async Task RefreshUpdateList()
    {
        try
        {
            var response = await _httpClient.GetAsync($"{ApiRoot}/{Endpoints.Updates}");
            if (response.IsSuccessStatusCode)
            {
                var updates = JsonSerializer.Deserialize<UpdateDescriptor[]>(
                    await response.Content.ReadAsStringAsync(),
                    _serializerOptions);

                if (updates != null)
                {
                    var previousIDs = Updates.Select(u => u.ID);
                    var currentIDs = updates.Select(u => u.ID);

                    var added = currentIDs.Where(u => !previousIDs.Contains(u));
                    var removed = previousIDs.Where(u => !currentIDs.Contains(u));

                    // TODO: handle removal

                    foreach (var update in updates)
                    {
                        if (added.Contains(update.ID))
                        {
                            Updates.Add(update);
                            OnUpdateAvailable?.Invoke(this, update);
                        }
                        else
                        {
                            var changed = false;
                            // check for changes - only fields that might differ are retrieved and applied
                            if (Updates[update.ID].Retrieved != update.Retrieved)
                            {
                                Updates[update.ID].Retrieved = update.Retrieved;
                                changed = true;
                            }
                            if (Updates[update.ID].Applied != update.Applied)
                            {
                                Updates[update.ID].Applied = update.Applied;
                                changed = true;
                            }
                            if (changed)
                            {
                                UpdateChanged?.Invoke(this, update);
                            }
                        }
                    }
                }
            }
        }
        catch (Exception ex)
        {
            // disconnect
            State = UpdateState.Disconnected;
        }
    }

    private async Task StateMonitorProc()
    {
        while (!_isDisposed)
        {
            if (_cancellationToken != null && _cancellationToken.Token.IsCancellationRequested) break;

            await GetDeviceInfo();
            await RefreshUpdateList();

            await Task.Delay(ServiceCheckPeriod);
        }
    }

    protected virtual void Dispose(bool disposing)
    {
        if (!_isDisposed)
        {
            if (disposing)
            {
                Stop();
            }

            _isDisposed = true;
        }
    }

    public void Dispose()
    {
        // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
        Dispose(disposing: true);
        GC.SuppressFinalize(this);
    }
}