namespace SaveForge.Core.Models;

public sealed class BackupRecord
{
    public required string OriginalPath { get; init; }
    public required string BackupPath { get; init; }
    public required DateTimeOffset Timestamp { get; init; }
}
