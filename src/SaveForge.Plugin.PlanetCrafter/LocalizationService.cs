using SaveForge.Plugin.PlanetCrafter.ViewModels;

namespace SaveForge.Plugin.PlanetCrafter;

/// <summary>
/// Loads item display names from the Planet Crafter localization file.
/// Keys in the file follow the pattern GROUP_NAME_{GId}; we extract English (first) values only.
/// Falls back to the raw GId string when the file cannot be found or parsed.
/// </summary>
internal sealed class LocalizationService
{
    private static readonly string[] SearchPaths =
    [
        // Steam default install on most Windows machines
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
            "Steam", "steamapps", "common", "Planet Crafter",
            "Planet Crafter_Data", "Resources", "LocalizationSpaceCrafter.txt"),
        // Steam on non-default drives (common alternate)
        Path.Combine(
            "D:", "SteamLibrary", "steamapps", "common", "Planet Crafter",
            "Planet Crafter_Data", "Resources", "LocalizationSpaceCrafter.txt"),
        Path.Combine(
            "E:", "SteamLibrary", "steamapps", "common", "Planet Crafter",
            "Planet Crafter_Data", "Resources", "LocalizationSpaceCrafter.txt"),
        Path.Combine(
            "F:", "SteamLibrary", "steamapps", "common", "Planet Crafter",
            "Planet Crafter_Data", "Resources", "LocalizationSpaceCrafter.txt"),
    ];

    private readonly Dictionary<string, string> _nameByGId;

    public IReadOnlyList<KnownItem> AllItems { get; }

    private LocalizationService(Dictionary<string, string> nameByGId)
    {
        _nameByGId = nameByGId;
        AllItems = nameByGId
            .Select(kv => new KnownItem(kv.Key, kv.Value))
            .OrderBy(x => x.DisplayName, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    public string GetItemName(string gId) =>
        _nameByGId.TryGetValue(gId, out var name) ? name : gId;

    public static LocalizationService TryLoad()
    {
        foreach (var path in SearchPaths)
        {
            if (!File.Exists(path))
            {
                continue;
            }

            try
            {
                return Load(path);
            }
            catch
            {
                // ignore and try next path
            }
        }
        return new LocalizationService(new Dictionary<string, string>(StringComparer.Ordinal));
    }

    private static LocalizationService Load(string path)
    {
        const string prefix = "GROUP_NAME_";
        var dict = new Dictionary<string, string>(StringComparer.Ordinal);

        foreach (var line in File.ReadAllLines(path))
        {
            var sep = line.IndexOf("**", StringComparison.Ordinal);
            if (sep < 0)
            {
                continue;
            }

            var key = line[..sep];
            if (!key.StartsWith(prefix, StringComparison.Ordinal))
            {
                continue;
            }

            var gId = key[prefix.Length..];
            var rest = line[(sep + 2)..];

            // Values are '*'-delimited; first entry is English
            var firstStar = rest.IndexOf('*');
            var english = firstStar >= 0 ? rest[..firstStar] : rest;
            english = english.Trim();

            if (!string.IsNullOrEmpty(english))
            {
                dict[gId] = english;
            }
        }

        return new LocalizationService(dict);
    }
}
