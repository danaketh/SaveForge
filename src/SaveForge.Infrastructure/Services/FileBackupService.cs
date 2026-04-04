using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.Infrastructure.Services;

public sealed class FileBackupService : IBackupService
{
    private readonly string _backupRootDir;

    public FileBackupService(string backupRootDir)
    {
        _backupRootDir = backupRootDir;
    }

    public async Task<BackupRecord> BackupAndWriteAsync(
        string filePath, string pluginId, string newContent, CancellationToken ct = default)
    {
        var timestamp = DateTimeOffset.UtcNow;
        var backupDir = Path.Combine(_backupRootDir, pluginId);
        Directory.CreateDirectory(backupDir);

        var fileName = Path.GetFileName(filePath);
        var tsStr = timestamp.ToString("yyyy-MM-ddTHH-mm-ss", System.Globalization.CultureInfo.InvariantCulture);
        var backupPath = Path.Combine(backupDir, $"{fileName}.{tsStr}.bak");

        // 1. Copy original to backup
        File.Copy(filePath, backupPath, overwrite: false);

        // 2. Write new content to temp file alongside original
        var tempPath = filePath + ".tmp";
        try
        {
            await File.WriteAllTextAsync(tempPath, newContent, ct);
            // 3. Atomic rename
            File.Move(tempPath, filePath, overwrite: true);
        }
        catch
        {
            // Clean up temp if it exists; original is untouched
            if (File.Exists(tempPath))
            {
                File.Delete(tempPath);
            }

            throw;
        }

        return new BackupRecord
        {
            OriginalPath = filePath,
            BackupPath = backupPath,
            Timestamp = timestamp
        };
    }
}
