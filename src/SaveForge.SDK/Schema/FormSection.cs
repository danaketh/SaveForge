namespace SaveForge.SDK.Schema;

public sealed class FormSection
{
    public string? Heading { get; init; }
    public FieldDefinition[] Fields { get; init; } = Array.Empty<FieldDefinition>();
}
