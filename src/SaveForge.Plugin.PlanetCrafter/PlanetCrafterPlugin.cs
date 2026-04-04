using System.Text;
using SaveForge.Plugin.PlanetCrafter.Parser;
using SaveForge.SDK;

namespace SaveForge.Plugin.PlanetCrafter;

public sealed class PlanetCrafterPlugin : IGameEditorPlugin
{
    public PluginManifest Manifest { get; } = new()
    {
        Id = "planet-crafter",
        Name = "Planet Crafter Save Editor",
        Version = "1.0.0",
        Author = "SaveForge Team",
        Description = "Edit saves for Planet Crafter.",
        MinHostVersion = "1.0.0",
        PluginAssembly = "SaveForge.Plugin.PlanetCrafter.dll"
    };

    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions = [".json"],
        DetectionPriority = 20,
        DefaultSaveDirectory = @"%APPDATA%\MijuGames\The Planet Crafter\Saves"
    };

    public (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header)
    {
        if (!file.Extension.Equals(".json", StringComparison.OrdinalIgnoreCase))
        {
            return (false, 0);
        }

        var text = Encoding.UTF8.GetString(header[..Math.Min(512, header.Length)]);
        return text.Contains("\"terraTokens\"", StringComparison.Ordinal)
            ? (true, Capabilities.DetectionPriority)
            : (false, 0);
    }

    public async Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default)
    {
        var content = await File.ReadAllTextAsync(filePath, ct);
        var save = PlanetCrafterSaveParser.Parse(content);
        return new PlanetCrafterDocument(filePath, save);
    }
}
