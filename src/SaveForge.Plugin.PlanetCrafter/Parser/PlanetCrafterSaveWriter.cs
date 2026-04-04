using System.Text;
using System.Text.Json;
using SaveForge.Plugin.PlanetCrafter.Models;

namespace SaveForge.Plugin.PlanetCrafter.Parser;

/// <summary>
/// Serializes a <see cref="PlanetCrafterSave"/> back to the Planet Crafter custom format.
///
/// Output format: 11 data groups joined by '@', each group's objects joined by "|\n".
/// The last object in each group has no trailing '|'. A final '@' is appended to match
/// the trailing empty group present in original save files.
/// </summary>
public static class PlanetCrafterSaveWriter
{
    private static readonly JsonSerializerOptions _opts = new() { WriteIndented = false };

    public static string Write(PlanetCrafterSave save)
    {
        var sb = new StringBuilder();

        sb.Append(Serialize(save.TerraTokens));
        sb.Append('@');
        sb.Append(Serialize(save.Planet));
        sb.Append('@');
        sb.Append(SerializeMany(save.Players));
        sb.Append('@');
        sb.Append(SerializeWorldObjects(save.WorldObjects));
        sb.Append('@');
        sb.Append(SerializeMany(save.Inventories));
        sb.Append('@');
        sb.Append(Serialize(save.Stats));
        sb.Append('@');
        sb.Append(SerializeMany(save.Messages));
        sb.Append('@');
        sb.Append(SerializeMany(save.StoryEvents));
        sb.Append('@');
        sb.Append(Serialize(save.DisplayInfo));
        sb.Append('@');
        sb.Append(SerializeMany(save.TerrainLayers));
        sb.Append('@');
        sb.Append(SerializeMany(save.PlanetInstances));
        sb.Append('@'); // trailing empty group

        return sb.ToString();
    }

    private static string Serialize<T>(T obj) => JsonSerializer.Serialize(obj, _opts);

    private static string SerializeMany<T>(T[] items)
    {
        if (items.Length == 0)
        {
            return "";
        }

        if (items.Length == 1)
        {
            return Serialize(items[0]);
        }

        return string.Join("|\n", items.Select(Serialize));
    }

    private static string SerializeWorldObjects(List<WorldObject> objects)
    {
        if (objects.Count == 0)
        {
            return "";
        }

        if (objects.Count == 1)
        {
            return objects[0].RawNode?.ToJsonString(_opts) ?? Serialize(objects[0]);
        }

        return string.Join("|\n", objects.Select(wo =>
            wo.RawNode?.ToJsonString(_opts) ?? Serialize(wo)));
    }
}
