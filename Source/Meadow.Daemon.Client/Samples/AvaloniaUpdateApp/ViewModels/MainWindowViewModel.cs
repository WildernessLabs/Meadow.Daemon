using Meadow.Daemon;
using Meadow.Update;
using ReactiveUI;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Reactive;

namespace AvaloniaUpdateApp.ViewModels
{
    public class MainWindowViewModel : ViewModelBase
    {
        private UpdateService? _service;
        private string _serviceAddress;
        private int _servicePort;

        private ObservableCollection<UpdateInfo> _updates = new ObservableCollection<UpdateInfo>();

        public ReactiveCommand<Unit, Unit> StartCommand { get; set; }
        public ReactiveCommand<Unit, Unit> RefreshCommand { get; set; }
        public ReactiveCommand<UpdateInfo, Unit> RetrieveCommand { get; set; }
        public ReactiveCommand<UpdateInfo, Unit> ApplyCommand { get; set; }

        public MainWindowViewModel()
        {
            // set the defaults
            ServicePort = 5000;
            ServiceAddress = "172.26.8.20";

            StartCommand = ReactiveCommand.Create(StartService);
            RefreshCommand = ReactiveCommand.Create(Refresh);
            RetrieveCommand = ReactiveCommand.Create<UpdateInfo>(RetrieveUpdate);
            ApplyCommand = ReactiveCommand.Create<UpdateInfo>(ApplyUpdate);
        }

        public DeviceInfo? DeviceInfo
        {
            get
            {
                if (_service == null) return null;
                return _service.DeviceInfo;
            }
        }

        public IEnumerable<UpdateInfo>? Updates
        {
            get => _updates;
        }

        public int ServicePort
        {
            get => _servicePort;
            set => this.RaiseAndSetIfChanged(ref _servicePort, value);
        }

        public string ServiceAddress
        {
            get => _serviceAddress;
            set
            {
                if (value == ServiceAddress) return;
                this.RaiseAndSetIfChanged(ref _serviceAddress, value);

                _service?.Dispose();

                _service = new UpdateService(ServiceAddress, ServicePort);
                _service.Connected += OnServiceConnected;
                _service.OnUpdateAvailable += OnUpdateAdded;
                _service.UpdateChanged += OnUpdateChanged;
            }
        }

        private void OnUpdateChanged(object? sender, UpdateInfo e)
        {
            this.RaisePropertyChanged(nameof(Updates));
        }

        private void OnUpdateAdded(object? sender, UpdateInfo e)
        {
            _updates.Add(e);
            this.RaisePropertyChanged(nameof(Updates));
        }

        private void Refresh()
        {
            this.RaisePropertyChanged(nameof(DeviceInfo));
            this.RaisePropertyChanged(nameof(Updates));
        }

        private void OnServiceConnected(object? sender, System.EventArgs e)
        {
            this.RaisePropertyChanged(nameof(DeviceInfo));
        }

        private void StartService()
        {
            _service?.Start();
        }

        private void RetrieveUpdate(UpdateInfo update)
        {
            _service?.RetrieveUpdate(update);
        }

        private void ApplyUpdate(UpdateInfo update)
        {
            _service?.ApplyUpdate(update);
        }
    }
}