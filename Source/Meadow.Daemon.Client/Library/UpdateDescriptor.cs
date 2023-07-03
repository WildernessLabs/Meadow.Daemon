using Meadow.Update;
using System.ComponentModel;
using System.Text.Json.Serialization;

namespace Meadow.Daemon;

internal record UpdateDescriptor : UpdateInfo, INotifyPropertyChanged
{
    [JsonPropertyName("MpakID")]
    public new string ID { get; set; }
}
