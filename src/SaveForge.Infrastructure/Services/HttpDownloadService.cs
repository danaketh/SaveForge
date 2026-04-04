namespace SaveForge.Infrastructure.Services;

public sealed class HttpDownloadService
{
    private readonly HttpClient _httpClient;

    public HttpDownloadService(HttpClient httpClient)
    {
        _httpClient = httpClient;
    }

    /// <summary>Downloads a URL to a local file, reporting bytes written.</summary>
    public async Task DownloadToFileAsync(
        string url, string destinationPath, IProgress<long>? progress = null, CancellationToken ct = default)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(destinationPath)!);

        if (url.StartsWith("file://", StringComparison.OrdinalIgnoreCase))
        {
            var sourcePath = new Uri(url).LocalPath;
            File.Copy(sourcePath, destinationPath, overwrite: true);
            progress?.Report(new FileInfo(destinationPath).Length);
            return;
        }

        using var response = await _httpClient.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(ct);
        await using var file = File.Create(destinationPath);

        var buffer = new byte[81920];
        long totalRead = 0;
        int read;
        while ((read = await stream.ReadAsync(buffer, ct)) > 0)
        {
            await file.WriteAsync(buffer.AsMemory(0, read), ct);
            totalRead += read;
            progress?.Report(totalRead);
        }
    }

    /// <summary>Fetches a URL as text, falling back to a local file:// URI.</summary>
    public async Task<string> FetchTextAsync(string url, CancellationToken ct = default)
    {
        if (url.StartsWith("file://", StringComparison.OrdinalIgnoreCase))
        {
            var path = new Uri(url).LocalPath;
            return await File.ReadAllTextAsync(path, ct);
        }
        return await _httpClient.GetStringAsync(url, ct);
    }
}
