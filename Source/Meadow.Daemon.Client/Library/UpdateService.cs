using System.Text.Json;

namespace Meadow.Daemon;

public partial class UpdateService : IDisposable
{
    public event EventHandler Connected = delegate { };
    public event EventHandler Disconnected = delegate { };
    public event EventHandler<UpdateDescriptor> UpdateAdded = delegate { };
    public event EventHandler<UpdateDescriptor> UpdateChanged = delegate { };

    private Task? _stateMonitor;
    private CancellationTokenSource? _cancellationToken;
    private bool _isDisposed;
    private HttpClient _httpClient;
    private JsonSerializerOptions _serializerOptions;
    private bool _isConnected;

    protected virtual TimeSpan ServiceCheckPeriod { get; } = TimeSpan.FromSeconds(5);
    protected virtual string ApiRoot { get; } = "/api";

    public DeviceInfo? DeviceInfo { get; private set; }
    public UpdateCollection Updates { get; }

    public UpdateService(string serviceAddress = "127.0.0.1", int servicePort = 5000)
    {
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

    public bool IsConnected
    {
        get => _isConnected;
        private set
        {
            if (value == IsConnected) return;
            _isConnected = value;
            if (IsConnected)
            {
                Connected?.Invoke(this, EventArgs.Empty);
            }
            else
            {
                Disconnected?.Invoke(this, EventArgs.Empty);
            }
        }
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

    public async Task BeginRetrieveUpdate(string updateID)
    {
        try
        {
            var existing = Updates[updateID];

            var payload = new JsonContent(new UpdateAction
            {
                Action = UpdateActions.Download
            });

            var response = await _httpClient.PutAsync(
                $"{ApiRoot}/{(Endpoints.UpdateAction.Replace("{id}", updateID))}",
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
            IsConnected = false;
        }
    }

    public async Task BeginApplyUpdate(string updateID)
    {
        try
        {
            var existing = Updates[updateID];

            var payload = new JsonContent(new UpdateAction
            {
                Action = UpdateActions.Apply
            });

            var response = await _httpClient.PutAsync(
                $"{ApiRoot}/{(Endpoints.UpdateAction.Replace("{id}", updateID))}",
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
            IsConnected = false;
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
                IsConnected = true;
                return info;
            }
        }
        catch (Exception ex)
        {
            // disconnect
            IsConnected = false;
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
                            UpdateAdded?.Invoke(this, update);
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
                IsConnected = true;
            }
        }
        catch (Exception ex)
        {
            // disconnect
            IsConnected = false;
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