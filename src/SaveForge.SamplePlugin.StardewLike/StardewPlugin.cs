using System.Text;
using System.Text.Json.Nodes;
using SaveForge.SDK;

namespace SaveForge.SamplePlugin.StardewLike;

public sealed class StardewPlugin : IGameEditorPlugin
{
    public PluginManifest Manifest { get; } = new()
    {
        Id = "stardew-like",
        Name = "StardewLike Save Editor",
        Version = "1.0.0",
        Author = "SaveForge Team",
        Description = "Edit saves for the StardewLike sample game.",
        MinHostVersion = "1.0.0",
        PluginAssembly = "SaveForge.SamplePlugin.StardewLike.dll"
    };

    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions = [".json"],
        DetectionPriority = 5,
        DefaultSaveDirectory = @"%APPDATA%\StardewValley\Saves"
    };

    public (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header)
    {
        if (!file.Extension.Equals(".json", StringComparison.OrdinalIgnoreCase))
        {
            return (false, 0);
        }

        // Look for "player" key in the first 256 bytes
        var text = Encoding.UTF8.GetString(header[..Math.Min(256, header.Length)]);
        return text.Contains("\"player\"", StringComparison.Ordinal)
            ? (true, Capabilities.DetectionPriority)
            : (false, 0);
    }

    public async Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default)
    {
        var json = await File.ReadAllTextAsync(filePath, ct);
        var root = JsonNode.Parse(json)
            ?? throw new InvalidOperationException("Failed to parse save file as JSON.");
        return new StardewSaveDocument(filePath, root);
    }
}
