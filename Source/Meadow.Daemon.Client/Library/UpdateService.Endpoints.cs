using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Meadow.Daemon;

public partial class UpdateService
{
    private static class Endpoints
    {
        public static string DeviceInfo => "info";
        public static string Updates => "updates";
        public static string UpdateAction => "updates/{id}";
    }

    private static class UpdateActions
    {
        public static string Apply => "apply";
        public static string Download => "download";
    }

    internal record UpdateAction
    {
        [JsonPropertyName("action")]
        public string Action { get; set; } = default!;
        [JsonPropertyName("pid")]
        public int Pid { get; set; }
        [JsonPropertyName("app_dir")]
        public string? AppDirectory { get; set; }
    }

    internal class JsonContent : StringContent
    {
        public JsonContent(object content)
            : base(JsonSerializer.Serialize(content), Encoding.UTF8, "text/json")
        {
        }
    }
}