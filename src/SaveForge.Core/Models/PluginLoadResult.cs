using SaveForge.SDK;

namespace SaveForge.Core.Models;

public abstract class PluginLoadResult
{
    private PluginLoadResult() { }

    public sealed class Succeeded : PluginLoadResult
    {
        public required IGameEditorPlugin Plugin { get; init; }
    }

    public sealed class Failed : PluginLoadResult
    {
        public required string Reason { get; init; }
    }

    public sealed class Incompatible : PluginLoadResult
    {
        public required string Reason { get; init; }
    }
}
