using System.Reflection;
using System.Runtime.Loader;

namespace SaveForge.PluginHost;

/// <summary>
/// Isolated AssemblyLoadContext for a single plugin.
/// Shared assemblies (Avalonia, SDK, System.*) fall through to the default context
/// so the host and plugin share the same type instances.
/// </summary>
public sealed class PluginLoadContext : AssemblyLoadContext
{
    private readonly AssemblyDependencyResolver _resolver;

    private static readonly string[] SharedPrefixes =
    [
        "Avalonia",
        "SaveForge.SDK",
        "System.",
        "Microsoft.",
        "netstandard",
        "mscorlib",
        "WindowsBase"
    ];

    public PluginLoadContext(string pluginAssemblyPath) : base(isCollectible: true)
    {
        _resolver = new AssemblyDependencyResolver(pluginAssemblyPath);
    }

    protected override Assembly? Load(AssemblyName assemblyName)
    {
        if (IsShared(assemblyName.Name))
        {
            return null; // delegate to Default context
        }

        var path = _resolver.ResolveAssemblyToPath(assemblyName);
        return path is not null ? LoadFromAssemblyPath(path) : null;
    }

    protected override IntPtr LoadUnmanagedDll(string unmanagedDllName)
    {
        var path = _resolver.ResolveUnmanagedDllToPath(unmanagedDllName);
        return path is not null ? LoadUnmanagedDllFromPath(path) : IntPtr.Zero;
    }

    private static bool IsShared(string? name)
    {
        if (name is null)
        {
            return false;
        }

        foreach (var prefix in SharedPrefixes)
        {
            if (name.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }
}
