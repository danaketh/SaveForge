namespace SaveForge.SDK.Schema;

/// <summary>Base class for all field definitions in a FormSchema.</summary>
public abstract class FieldDefinition
{
    /// <summary>Dot-path key used to get/set the value in ISaveDocument (e.g. "player.stats.health").</summary>
    public required string Key { get; init; }

    public required string Label { get; init; }

    public string? Description { get; init; }

    public bool IsReadOnly { get; init; }

    public bool IsRequired { get; init; }

    /// <summary>When set, the field is only shown when the rule evaluates to true.</summary>
    public VisibilityRule? VisibilityRule { get; init; }
}

public sealed class TextField : FieldDefinition
{
    public int? MaxLength { get; init; }
    public string? RegexPattern { get; init; }
}

public sealed class NumericField : FieldDefinition
{
    public double? Min { get; init; }
    public double? Max { get; init; }
    public int DecimalPlaces { get; init; }
}

public sealed class CheckboxField : FieldDefinition;

public sealed class EnumField : FieldDefinition
{
    public EnumOption[] Options { get; init; } = Array.Empty<EnumOption>();
}

public sealed class ReadOnlyField : FieldDefinition;

public sealed class ListField : FieldDefinition
{
    /// <summary>Template defining the shape of each list item.</summary>
    public required FieldDefinition ItemSchema { get; init; }
}

public sealed class ObjectSectionField : FieldDefinition
{
    public FieldDefinition[] Fields { get; init; } = Array.Empty<FieldDefinition>();
}
