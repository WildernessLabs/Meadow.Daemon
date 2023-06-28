using System.Text.Json.Serialization;

namespace Meadow.Daemon;

public record UpdateDescriptor
{
    [JsonPropertyName("MpakID")]
    public string ID { get; set; }
    public DateTimeOffset PublishedOn { get; set; }
    public int UpdateType { get; set; }
    public string Version { get; set; }
    public int DownloadSize { get; set; }
    public string Summary { get; set; }
    public string Detail { get; set; }
    public bool Retrieved { get; set; }
    public bool Applied { get; set; }
}
