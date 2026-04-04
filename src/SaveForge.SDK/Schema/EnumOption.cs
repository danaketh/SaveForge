namespace SaveForge.SDK.Schema;

/// <summary>One option in an EnumField dropdown.</summary>
public sealed class EnumOption
{
    public required string Value { get; init; }
    public required string Display { get; init; }
}
