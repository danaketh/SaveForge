namespace SaveForge.Core.Models;

public sealed class AppSettings
{
    public string CatalogUrl { get; set; } = AppSettingsDefaults.CatalogUrl;
    public string PluginDirectory { get; set; } = AppSettingsDefaults.PluginDirectory;
    public string BackupDirectory { get; set; } = AppSettingsDefaults.BackupDirectory;
    public bool CheckUpdatesOnStartup { get; set; } = true;
    public Dictionary<string, string> PluginSaveDirectories { get; set; } = [];
}

public static class AppSettingsDefaults
{
    private static readonly string AppData =
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "SaveForge");

#if DEBUG
    public static string PluginDirectory { get; } = Path.Combine(AppContext.BaseDirectory, "_devplugins");
#else
    public static string PluginDirectory { get; } = Path.Combine(AppData, "plugins");
#endif
    public static string BackupDirectory { get; } = Path.Combine(AppData, "backups");
    public static string SettingsPath { get; } = Path.Combine(AppData, "settings.json");
    public static string CatalogCachePath { get; } = Path.Combine(AppData, "catalog-cache.json");

#if DEBUG
    public static string CatalogUrl { get; } =
        new Uri(Path.Combine(AppData, "catalog.json")).AbsoluteUri;
#else
    public static string CatalogUrl { get; } =
        "https://raw.githubusercontent.com/YOUR_ORG/YOUR_REPO/main/catalog.json";
#endif
}
