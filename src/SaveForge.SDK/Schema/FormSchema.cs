namespace SaveForge.SDK.Schema;

/// <summary>Top-level schema that drives the generic form renderer.</summary>
public sealed class FormSchema
{
    public FormPage[] Pages { get; init; } = Array.Empty<FormPage>();
}
