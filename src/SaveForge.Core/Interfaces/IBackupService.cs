using SaveForge.Core.Models;

namespace SaveForge.Core.Interfaces;

public interface IBackupService
{
    /// <summary>Creates a timestamped backup of filePath, then atomically writes newContent to filePath.</summary>
    Task<BackupRecord> BackupAndWriteAsync(string filePath, string pluginId, string newContent, CancellationToken ct = default);
}
