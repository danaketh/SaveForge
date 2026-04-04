using SaveForge.SDK.Schema;
using Xunit;

namespace SaveForge.Tests;

public class VisibilityRuleTests
{
    [Fact]
    public void Equals_MatchingString_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.Equals, Value = "admin" };
        Assert.True(rule.Evaluate("admin"));
    }

    [Fact]
    public void Equals_NonMatchingString_ReturnsFalse()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.Equals, Value = "admin" };
        Assert.False(rule.Evaluate("user"));
    }

    [Fact]
    public void NotEquals_DifferentValue_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.NotEquals, Value = "admin" };
        Assert.True(rule.Evaluate("user"));
    }

    [Fact]
    public void GreaterThan_LargerInt_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.GreaterThan, Value = 5 };
        Assert.True(rule.Evaluate(10));
    }

    [Fact]
    public void LessThan_SmallerInt_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.LessThan, Value = 10 };
        Assert.True(rule.Evaluate(3));
    }

    [Fact]
    public void Equals_NullBothSides_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.Equals, Value = null! };
        Assert.True(rule.Evaluate(null));
    }
}
