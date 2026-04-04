using System.Text.RegularExpressions;
using SaveForge.SDK;

namespace SaveForge.Security;

public static partial class ManifestValidator
{
    private static readonly Regex IdPattern = MyRegex();

    /// <summary>Returns a list of validation error messages. Empty = valid.</summary>
    public static IReadOnlyList<string> Validate(PluginManifest manifest)
    {
        var errors = new List<string>();

        if (string.IsNullOrWhiteSpace(manifest.Id))
        {
            errors.Add("Id is required.");
        }
        else if (!IdPattern.IsMatch(manifest.Id))
        {
            errors.Add("Id must contain only lowercase letters, digits, and hyphens.");
        }

        if (string.IsNullOrWhiteSpace(manifest.Name))
        {
            errors.Add("Name is required.");
        }

        if (!TryParseVersion(manifest.Version, out _))
        {
            errors.Add($"Version '{manifest.Version}' is not a valid version string.");
        }

        if (string.IsNullOrWhiteSpace(manifest.Author))
        {
            errors.Add("Author is required.");
        }

        if (!TryParseVersion(manifest.MinHostVersion, out _))
        {
            errors.Add($"MinHostVersion '{manifest.MinHostVersion}' is not a valid version string.");
        }

        if (manifest.MaxHostVersion is not null && !TryParseVersion(manifest.MaxHostVersion, out _))
        {
            errors.Add($"MaxHostVersion '{manifest.MaxHostVersion}' is not a valid version string.");
        }

        if (string.IsNullOrWhiteSpace(manifest.PluginAssembly))
        {
            errors.Add("PluginAssembly is required.");
        }

        return errors;
    }

    private static bool TryParseVersion(string? s, out Version version)
    {
        version = new Version();
        if (string.IsNullOrWhiteSpace(s))
        {
            return false;
        }

        return Version.TryParse(s, out version!);
    }

    [GeneratedRegex("^[a-z0-9-]+$", RegexOptions.Compiled)]
    private static partial Regex MyRegex();
}
