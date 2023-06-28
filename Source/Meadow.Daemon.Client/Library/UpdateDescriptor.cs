using System.ComponentModel;
using System.Text.Json.Serialization;

namespace Meadow.Daemon;

public record UpdateDescriptor : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler PropertyChanged = delegate { };

    private bool _applied;
    private bool _retrieved;

    [JsonPropertyName("MpakID")]
    public string ID { get; set; }
    public DateTimeOffset PublishedOn { get; set; }
    public int UpdateType { get; set; }
    public string Version { get; set; }
    public int DownloadSize { get; set; }
    public string Summary { get; set; }
    public string Detail { get; set; }

    public bool Retrieved
    {
        get => _retrieved;
        set
        {
            if (value == Retrieved) return;
            _retrieved = value;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Retrieved)));
        }
    }

    public bool Applied
    {
        get => _applied;
        set
        {
            if (value == Applied) return;
            _applied = value;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Applied)));
        }
    }
}
