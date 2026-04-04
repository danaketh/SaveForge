namespace SaveForge.SDK.Schema;

public enum VisibilityOperator { Equals, NotEquals, GreaterThan, LessThan }

/// <summary>
/// Makes a field visible only when another field's value satisfies a condition.
/// </summary>
public sealed class VisibilityRule
{
    public required string DependsOnKey { get; init; }
    public required VisibilityOperator Operator { get; init; }
    public required object Value { get; init; }

    /// <summary>Evaluates whether the field should be visible given the current value of DependsOnKey.</summary>
    public bool Evaluate(object? currentValue)
    {
        return Operator switch
        {
            VisibilityOperator.Equals => AreEqual(currentValue, Value),
            VisibilityOperator.NotEquals => !AreEqual(currentValue, Value),
            VisibilityOperator.GreaterThan => Compare(currentValue, Value) > 0,
            VisibilityOperator.LessThan => Compare(currentValue, Value) < 0,
            _ => true
        };
    }

    private static bool AreEqual(object? a, object? b)
    {
        if (a is null && b is null)
        {
            return true;
        }

        if (a is null || b is null)
        {
            return false;
        }

        try
        { return Convert.ChangeType(a, b.GetType(), System.Globalization.CultureInfo.InvariantCulture).Equals(b); }
        catch { return a.Equals(b); }
    }

    private static int Compare(object? a, object? b)
    {
        if (a is IComparable ca && b is not null)
        {
            try
            { return ca.CompareTo(Convert.ChangeType(b, a.GetType(), System.Globalization.CultureInfo.InvariantCulture)); }
            catch { return 0; }
        }
        return 0;
    }
}
