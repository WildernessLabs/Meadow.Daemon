namespace Meadow.Daemon;

public record DeviceInfo
{
    public string Service { get; set; }
    public string Version { get; set; }
    public string Status { get; set; }
}
