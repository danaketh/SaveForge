using System.Text.Json;
using System.Text.Json.Nodes;
using SaveForge.Plugin.PlanetCrafter.Models;

namespace SaveForge.Plugin.PlanetCrafter.Parser;

/// <summary>
/// Parses the Planet Crafter custom save format into a <see cref="PlanetCrafterSave"/>.
///
/// Format: Groups are separated by '@'. Within each group, JSON objects are separated
/// by '|' (with optional surrounding whitespace/newlines). The last object in a group
/// has no trailing '|'. There are always at least 11 groups (index 0-10); a trailing
/// empty group (index 11) is normal and ignored.
/// </summary>
public static class PlanetCrafterSaveParser
{
    private static readonly JsonSerializerOptions _opts = new()
    {
        PropertyNameCaseInsensitive = false
    };

    public static PlanetCrafterSave Parse(string content)
    {
        ArgumentNullException.ThrowIfNull(content);

        var groups = content.Split('@');
        if (groups.Length < 11)
        {
            throw new InvalidDataException(
                $"Expected at least 11 groups separated by '@', found {groups.Length}.");
        }

        return new PlanetCrafterSave
        {
            TerraTokens = ParseSingle<TerraTokensData>(groups[0]),
            Planet = ParseSingle<PlanetData>(groups[1]),
            Players = ParseMany<PlayerData>(groups[2]),
            WorldObjects = ParseWorldObjects(groups[3]),
            Inventories = ParseMany<InventoryData>(groups[4]),
            Stats = ParseSingle<GlobalStats>(groups[5]),
            Messages = ParseMany<MessageData>(groups[6]),
            StoryEvents = ParseMany<StoryEventData>(groups[7]),
            DisplayInfo = ParseSingle<SaveDisplayInfo>(groups[8]),
            TerrainLayers = ParseMany<TerrainLayer>(groups[9]),
            PlanetInstances = ParseMany<PlanetInstance>(groups[10]),
        };
    }

    private static T ParseSingle<T>(string group)
    {
        var json = group.Trim();
        return JsonSerializer.Deserialize<T>(json, _opts)
            ?? throw new InvalidDataException(
                $"Failed to deserialize group as {typeof(T).Name}: {json[..Math.Min(80, json.Length)]}");
    }

    private static T[] ParseMany<T>(string group)
    {
        var items = SplitObjects(group);
        if (items.Length == 0)
        {
            return Array.Empty<T>();
        }

        var result = new T[items.Length];
        for (int i = 0; i < items.Length; i++)
        {
            result[i] = JsonSerializer.Deserialize<T>(items[i], _opts)
                ?? throw new InvalidDataException(
                    $"Failed to deserialize item {i} as {typeof(T).Name}.");
        }
        return result;
    }

    private static List<WorldObject> ParseWorldObjects(string group)
    {
        var items = SplitObjects(group);
        if (items.Length == 0)
        {
            return [];
        }

        var result = new List<WorldObject>(items.Length);
        for (int i = 0; i < items.Length; i++)
        {
            var node = JsonNode.Parse(items[i]) as JsonObject
                ?? throw new InvalidDataException($"World object {i} is not a JSON object.");

            result.Add(new WorldObject
            {
                Id = node["id"]?.GetValue<int>() ?? 0,
                GId = node["gId"]?.GetValue<string>() ?? "",
                RawNode = node,
            });
        }
        return result;
    }

    /// <summary>Splits a group string into individual JSON object strings.</summary>
    private static string[] SplitObjects(string group) =>
        group.Split('|')
             .Select(s => s.Trim())
             .Where(s => s.Length > 0)
             .ToArray();
}
