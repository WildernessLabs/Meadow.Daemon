using Meadow.Daemon;
using ReactiveUI;
using System.Collections.Generic;
using System.Reactive;

namespace AvaloniaUpdateApp.ViewModels
{
    public class MainWindowViewModel : ViewModelBase
    {
        private UpdateService? _service;
        private string _serviceAddress;
        private int _servicePort;

        public ReactiveCommand<Unit, Unit> StartCommand { get; set; }
        public ReactiveCommand<Unit, Unit> RefreshCommand { get; set; }

        public MainWindowViewModel()
        {
            // set the defaults
            ServicePort = 5000;
            ServiceAddress = "172.26.8.20";

            StartCommand = ReactiveCommand.Create(StartService);
            RefreshCommand = ReactiveCommand.Create(Refresh);
        }

        public DeviceInfo? DeviceInfo
        {
            get
            {
                if (_service == null) return null;
                return _service.DeviceInfo;
            }
        }

        public IEnumerable<UpdateDescriptor>? Updates
        {
            get
            {
                if (_service == null) return null;
                return _service.Updates;
            }
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
            }
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
    }
}