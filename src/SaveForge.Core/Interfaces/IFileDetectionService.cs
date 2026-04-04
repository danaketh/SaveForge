using SaveForge.SDK;

namespace SaveForge.Core.Interfaces;

public sealed record DetectionResult(IGameEditorPlugin Plugin, int Priority);

public interface IFileDetectionService
{
    /// <summary>Returns all plugins that can open the file, sorted by priority descending.</summary>
    Task<IReadOnlyList<DetectionResult>> DetectAsync(string filePath, CancellationToken ct = default);
}
