using System.Security.Cryptography;

namespace SaveForge.Security;

public static class HashVerifier
{
    /// <summary>
    /// Returns true if the SHA-256 of the file at filePath matches expectedSha256 (hex, case-insensitive).
    /// </summary>
    public static async Task<bool> VerifyAsync(string filePath, string expectedSha256, CancellationToken ct = default)
    {
        using var sha = SHA256.Create();
        await using var stream = File.OpenRead(filePath);
        var hash = await sha.ComputeHashAsync(stream, ct);
        var actual = Convert.ToHexString(hash);
        return string.Equals(actual, expectedSha256, StringComparison.OrdinalIgnoreCase);
    }
}
