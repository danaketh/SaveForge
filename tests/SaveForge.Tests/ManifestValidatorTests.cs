using SaveForge.SDK;
using SaveForge.Security;
using Xunit;

namespace SaveForge.Tests;

public class ManifestValidatorTests
{
    private static PluginManifest Make(
        string id = "my-plugin", string name = "My Plugin", string version = "1.0.0",
        string author = "Author", string minHost = "1.0.0", string assembly = "MyPlugin.dll") => new()
        {
            Id = id,
            Name = name,
            Version = version,
            Author = author,
            MinHostVersion = minHost,
            PluginAssembly = assembly
        };

    [Fact]
    public void Validate_ValidManifest_ReturnsNoErrors()
    {
        Assert.Empty(ManifestValidator.Validate(Make()));
    }

    [Fact]
    public void Validate_EmptyId_ReturnsError()
    {
        Assert.Contains(ManifestValidator.Validate(Make(id: "")), e => e.Contains("Id"));
    }

    [Fact]
    public void Validate_InvalidIdChars_ReturnsError()
    {
        Assert.Contains(ManifestValidator.Validate(Make(id: "My Plugin!")), e => e.Contains("Id"));
    }

    [Fact]
    public void Validate_InvalidVersion_ReturnsError()
    {
        Assert.Contains(ManifestValidator.Validate(Make(version: "not-a-version")), e => e.Contains("Version"));
    }

    [Fact]
    public void Validate_EmptyPluginAssembly_ReturnsError()
    {
        Assert.Contains(ManifestValidator.Validate(Make(assembly: "")), e => e.Contains("PluginAssembly"));
    }
}
