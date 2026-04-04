using System.Globalization;

namespace SaveForge.Core.Models;

public sealed record SaveFileInfo(
    string FilePath,
    string FileName,
    DateTimeOffset LastModified,
    long SizeBytes)
{
    public string LastModifiedDisplay => LastModified.ToString("MMM d, yyyy", CultureInfo.InvariantCulture);

    public string SizeDisplay => SizeBytes < 1_048_576
        ? $"{SizeBytes / 1024.0:0.0} KB"
        : $"{SizeBytes / 1_048_576.0:0.0} MB";
}
