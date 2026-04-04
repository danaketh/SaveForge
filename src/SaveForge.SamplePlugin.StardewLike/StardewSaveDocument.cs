using System.Text.Json;
using System.Text.Json.Nodes;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.SamplePlugin.StardewLike;

public sealed class StardewSaveDocument : ISaveDocument
{
    private JsonNode _root;
    private bool _isDirty;

    private static readonly JsonSerializerOptions PrettyJson = new() { WriteIndented = true };

    public StardewSaveDocument(string filePath, JsonNode root)
    {
        FilePath = filePath;
        _root = root;
    }

    public string FilePath { get; }
    public bool IsDirty => _isDirty;

    public object? GetValue(string dotPath)
    {
        var node = Navigate(dotPath);
        return node switch
        {
            JsonValue v when v.TryGetValue<string>(out var s) => s,
            JsonValue v when v.TryGetValue<int>(out var i) => i,
            JsonValue v when v.TryGetValue<long>(out var l) => l,
            JsonValue v when v.TryGetValue<double>(out var d) => d,
            JsonValue v when v.TryGetValue<bool>(out var b) => b,
            JsonArray arr => arr,
            _ => null
        };
    }

    public void SetValue(string dotPath, object? value)
    {
        var parts = dotPath.Split('.', StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length == 0)
        {
            return;
        }

        JsonNode? current = _root;
        for (int i = 0; i < parts.Length - 1; i++)
        {
            if (current is JsonObject obj)
            {
                current = obj[parts[i]];
            }
            else
            {
                return;
            }
        }

        if (current is JsonObject parent)
        {
            parent[parts[^1]] = value switch
            {
                null => null,
                string s => JsonValue.Create(s),
                int n => JsonValue.Create(n),
                long l => JsonValue.Create(l),
                double d => JsonValue.Create(d),
                bool b => JsonValue.Create(b),
                _ => JsonValue.Create(value.ToString())
            };
            _isDirty = true;
        }
    }

    public IEditorDefinition GetEditorDefinition() =>
        new SchemaEditorDefinition(StardewSchema.Build());

    public async Task SaveAsync(CancellationToken ct = default)
    {
        var json = _root.ToJsonString(PrettyJson);
        await File.WriteAllTextAsync(FilePath, json, ct);
        _isDirty = false;
    }

    private JsonNode? Navigate(string dotPath)
    {
        var parts = dotPath.Split('.', StringSplitOptions.RemoveEmptyEntries);
        JsonNode? current = _root;
        foreach (var part in parts)
        {
            if (current is JsonObject obj)
            {
                current = obj[part];
            }
            else
            {
                return null;
            }
        }
        return current;
    }
}

internal sealed class SchemaEditorDefinition : IEditorDefinition
{
    public SchemaEditorDefinition(FormSchema schema) => Schema = schema;
    public EditorKind Kind => EditorKind.Schema;
    public FormSchema? Schema { get; }
    public ICustomEditorViewFactory? ViewFactory => null;
}
