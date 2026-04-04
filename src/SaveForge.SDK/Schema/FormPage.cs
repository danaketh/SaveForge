namespace SaveForge.SDK.Schema;

public sealed class FormPage
{
    public required string Title { get; init; }
    public FormSection[] Sections { get; init; } = Array.Empty<FormSection>();
}
