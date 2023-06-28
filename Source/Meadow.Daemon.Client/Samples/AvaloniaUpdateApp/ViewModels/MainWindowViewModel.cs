using Meadow.Daemon;
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

        private ObservableCollection<UpdateDescriptor> _updates = new ObservableCollection<UpdateDescriptor>();

        public ReactiveCommand<Unit, Unit> StartCommand { get; set; }
        public ReactiveCommand<Unit, Unit> RefreshCommand { get; set; }
        public ReactiveCommand<UpdateDescriptor, Unit> RetrieveCommand { get; set; }
        public ReactiveCommand<UpdateDescriptor, Unit> ApplyCommand { get; set; }

        public MainWindowViewModel()
        {
            // set the defaults
            ServicePort = 5000;
            ServiceAddress = "172.26.8.20";

            StartCommand = ReactiveCommand.Create(StartService);
            RefreshCommand = ReactiveCommand.Create(Refresh);
            RetrieveCommand = ReactiveCommand.Create<UpdateDescriptor>(RetrieveUpdate);
            ApplyCommand = ReactiveCommand.Create<UpdateDescriptor>(ApplyUpdate);
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
                _service.UpdateAdded += OnUpdateAdded;
                _service.UpdateChanged += OnUpdateChanged;
            }
        }

        private void OnUpdateChanged(object? sender, UpdateDescriptor e)
        {
            this.RaisePropertyChanged(nameof(Updates));
        }

        private void OnUpdateAdded(object? sender, UpdateDescriptor e)
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

        private void RetrieveUpdate(UpdateDescriptor update)
        {
            _service?.BeginRetrieveUpdate(update.ID);
        }

        private void ApplyUpdate(UpdateDescriptor update)
        {
            _service?.BeginApplyUpdate(update.ID);
        }
    }
}