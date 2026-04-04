namespace SaveForge.Infrastructure.Services;

/// <summary>Expands environment-variable tokens in directory path strings.</summary>
public static class PathTokenExpander
{
    private static readonly (string Token, Func<string> Resolver)[] _knownTokens =
    [
        ("%APPDATA%",      () => Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData)),
        ("%LOCALAPPDATA%", () => Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData)),
        ("%USERPROFILE%",  () => Environment.GetFolderPath(Environment.SpecialFolder.UserProfile)),
        ("%TEMP%",         () => Path.GetTempPath().TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)),
    ];

    /// <summary>
    /// Replaces known tokens in <paramref name="path"/> with their runtime values.
    /// Unrecognised tokens are left as-is. Leading <c>~</c> is treated as %USERPROFILE%.
    /// </summary>
    public static string Expand(string path)
    {
        if (string.IsNullOrEmpty(path))
        {
            return path;
        }

        // Handle leading tilde
        if (path.StartsWith("~/", StringComparison.Ordinal) || path.StartsWith(@"~\", StringComparison.Ordinal))
        {
            var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            path = home + path[1..];
        }

        foreach (var (token, resolver) in _knownTokens)
        {
            if (path.Contains(token, StringComparison.OrdinalIgnoreCase))
            {
                path = path.Replace(token, resolver(), StringComparison.OrdinalIgnoreCase);
            }
        }

        return path;
    }
}
