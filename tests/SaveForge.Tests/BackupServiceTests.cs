using SaveForge.Infrastructure.Services;
using Xunit;

namespace SaveForge.Tests;

public class BackupServiceTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-test-" + Guid.NewGuid());
    private readonly string _backupDir;

    public BackupServiceTests()
    {
        Directory.CreateDirectory(_tempDir);
        _backupDir = Path.Combine(_tempDir, "backups");
        Directory.CreateDirectory(_backupDir);
    }

    [Fact]
    public async Task BackupAndWrite_CreatesBackupFile()
    {
        var originalPath = Path.Combine(_tempDir, "save.json");
        await File.WriteAllTextAsync(originalPath, """{"v":1}""");
        var svc = new FileBackupService(_backupDir);

        var record = await svc.BackupAndWriteAsync(originalPath, "test-plugin", """{"v":2}""");

        Assert.True(File.Exists(record.BackupPath));
        Assert.Equal("""{"v":1}""", await File.ReadAllTextAsync(record.BackupPath));
    }

    [Fact]
    public async Task BackupAndWrite_WritesNewContentToOriginal()
    {
        var originalPath = Path.Combine(_tempDir, "save2.json");
        await File.WriteAllTextAsync(originalPath, """{"v":1}""");
        var svc = new FileBackupService(_backupDir);

        await svc.BackupAndWriteAsync(originalPath, "test-plugin", """{"v":2}""");

        Assert.Equal("""{"v":2}""", await File.ReadAllTextAsync(originalPath));
    }

    [Fact]
    public async Task BackupAndWrite_OriginalPreservedIfWriteFails()
    {
        var originalPath = Path.Combine(_tempDir, "save3.json");
        await File.WriteAllTextAsync(originalPath, """{"v":1}""");
        // Use an existing file as the backup "directory" — CreateDirectory will throw
        var notADir = Path.Combine(_tempDir, "not-a-dir.txt");
        await File.WriteAllTextAsync(notADir, "blocker");
        var svc = new FileBackupService(notADir);

        await Assert.ThrowsAnyAsync<Exception>(() =>
            svc.BackupAndWriteAsync(originalPath, "test-plugin", """{"v":2}"""));

        Assert.Equal("""{"v":1}""", await File.ReadAllTextAsync(originalPath));
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);
}
