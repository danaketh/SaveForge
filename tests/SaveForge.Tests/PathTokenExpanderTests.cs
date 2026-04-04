using SaveForge.Infrastructure.Services;
using Xunit;

namespace SaveForge.Tests;

public class PathTokenExpanderTests
{
    [Fact]
    public void Expand_NoTokens_ReturnsSamePath()
    {
        var result = PathTokenExpander.Expand(@"C:\Users\dan\saves");
        Assert.Equal(@"C:\Users\dan\saves", result);
    }

    [Fact]
    public void Expand_AppDataToken_ReplacesWithFolder()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        var result = PathTokenExpander.Expand(@"%APPDATA%\MyGame\Saves");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
        Assert.EndsWith(@"\MyGame\Saves", result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_LocalAppDataToken_ReplacesWithFolder()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var result = PathTokenExpander.Expand(@"%LOCALAPPDATA%\Game");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_UserProfileToken_ReplacesWithFolder()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var result = PathTokenExpander.Expand(@"%USERPROFILE%\Documents\Saves");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_TempToken_ReplacesWithTempPath()
    {
        var expected = Path.GetTempPath().TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var result = PathTokenExpander.Expand(@"%TEMP%\saves");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_LeadingTilde_ReplacesWithUserProfile()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var result = PathTokenExpander.Expand(@"~/saves/game");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_UnknownToken_LeavesItAsIs()
    {
        var result = PathTokenExpander.Expand(@"%UNKNOWN_TOKEN%\saves");
        Assert.Equal(@"%UNKNOWN_TOKEN%\saves", result);
    }

    [Fact]
    public void Expand_EmptyString_ReturnsEmptyString()
    {
        var result = PathTokenExpander.Expand(string.Empty);
        Assert.Equal(string.Empty, result);
    }

    [Fact]
    public void Expand_TokenCaseInsensitive_ReplacesCorrectly()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        var result = PathTokenExpander.Expand(@"%appdata%\Game");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }
}
