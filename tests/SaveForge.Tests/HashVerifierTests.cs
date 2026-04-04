using System.Security.Cryptography;
using SaveForge.Security;
using Xunit;

namespace SaveForge.Tests;

public class HashVerifierTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName();

    [Fact]
    public async Task VerifyAsync_CorrectHash_ReturnsTrue()
    {
        await File.WriteAllTextAsync(_tempFile, "hello world");
        var expected = await ComputeHashAsync(_tempFile);

        Assert.True(await HashVerifier.VerifyAsync(_tempFile, expected));
    }

    [Fact]
    public async Task VerifyAsync_WrongHash_ReturnsFalse()
    {
        await File.WriteAllTextAsync(_tempFile, "hello world");

        Assert.False(await HashVerifier.VerifyAsync(_tempFile, "0000000000000000000000000000000000000000000000000000000000000000"));
    }

    [Fact]
    public async Task VerifyAsync_CaseInsensitive_ReturnsTrue()
    {
        await File.WriteAllTextAsync(_tempFile, "test");
        var upper = (await ComputeHashAsync(_tempFile)).ToUpperInvariant();

        Assert.True(await HashVerifier.VerifyAsync(_tempFile, upper));
    }

    private static async Task<string> ComputeHashAsync(string path)
    {
        using var sha = SHA256.Create();
        await using var stream = File.OpenRead(path);
        var bytes = await sha.ComputeHashAsync(stream);
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }

    public void Dispose()
    {
        if (File.Exists(_tempFile))
        {
            File.Delete(_tempFile);
        }
    }
}
