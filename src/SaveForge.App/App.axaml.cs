using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using SaveForge.App.ViewModels;
using SaveForge.App.Views;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.PluginHost;
using SaveForge.Store;

namespace SaveForge.App;

public class App : Application
{
    private ServiceProvider? _services;

    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            _services = BuildServices();

            // Load plugins on startup
            var hostSvc = _services.GetRequiredService<PluginHostService>();
            var settings = _services.GetRequiredService<AppSettings>();
            hostSvc.LoadAll(settings.PluginDirectory);

            var mainVm = _services.GetRequiredService<MainViewModel>();
            var window = new MainWindow { DataContext = mainVm };
            desktop.MainWindow = window;
        }
        base.OnFrameworkInitializationCompleted();
    }

    private static ServiceProvider BuildServices()
    {
        var services = new ServiceCollection();

        // Logging
        services.AddLogging(b => b.AddConsole().SetMinimumLevel(LogLevel.Information));

        // Settings (loaded synchronously at startup)
        var settingsService = new SettingsService(AppSettingsDefaults.SettingsPath);
        var settings = settingsService.LoadAsync().GetAwaiter().GetResult();
        services.AddSingleton(settings);
        services.AddSingleton(settingsService);

        // Infrastructure
        services.AddSingleton<HttpClient>();
        services.AddSingleton<HttpDownloadService>();
        services.AddSingleton<IBackupService>(sp =>
            new FileBackupService(settings.BackupDirectory));
        services.AddSingleton<ISaveDiscoveryService, SaveDiscoveryService>();

        // Plugin Host
        services.AddSingleton<PluginRegistry>();
        services.AddSingleton<IPluginRegistry>(sp => sp.GetRequiredService<PluginRegistry>());
        services.AddSingleton<PluginLoader>(sp =>
            new PluginLoader(sp.GetRequiredService<ILogger<PluginLoader>>(), "1.0.0"));
        services.AddSingleton<PluginHostService>();
        services.AddSingleton<IFileDetectionService, FileDetectionService>();

        // Store
        services.AddSingleton<ICatalogService>(sp =>
        {
            IPluginRegistry registry = sp.GetRequiredService<IPluginRegistry>();
            var installedIds = new HashSet<string>(registry.GetAll().Select(p => p.Manifest.Id));
            return new CatalogService(
                sp.GetRequiredService<HttpDownloadService>(),
                settings.CatalogUrl,
                AppSettingsDefaults.CatalogCachePath,
                installedIds);
        });
        services.AddSingleton<IPluginInstaller>(sp =>
            new PluginInstaller(
                sp.GetRequiredService<HttpDownloadService>(),
                sp.GetRequiredService<ICatalogService>(),
                sp.GetRequiredService<PluginRegistry>(),
                sp.GetRequiredService<PluginLoader>(),
                sp.GetRequiredService<ILogger<PluginInstaller>>(),
                settings.PluginDirectory));
        services.AddSingleton<IUpdateChecker>(sp =>
            new UpdateChecker(
                sp.GetRequiredService<ICatalogService>(),
                sp.GetRequiredService<IPluginRegistry>()));

        // App ViewModels
        services.AddSingleton<StatusBarViewModel>();
        services.AddSingleton<InfoBarViewModel>();
        services.AddSingleton<AboutViewModel>();
        services.AddSingleton<HomeViewModel>(sp =>
            new HomeViewModel(
                sp.GetRequiredService<IPluginRegistry>(),
                sp.GetRequiredService<ISaveDiscoveryService>(),
                sp.GetRequiredService<AppSettings>(),
                sp.GetRequiredService<SettingsService>(),
                page => sp.GetRequiredService<MainViewModel>().NavigateToCommand.Execute(page),
                async path =>
                {
                    var infoBar = sp.GetRequiredService<InfoBarViewModel>();
                    try
                    {
                        var detection = sp.GetRequiredService<IFileDetectionService>();
                        var results = await detection.DetectAsync(path);
                        if (results.Count == 0)
                        {
                            infoBar.Show("No plugin recognised this save file.");
                            return null;
                        }

                        var doc = await results[0].Plugin.OpenAsync(path);
                        if (doc is null)
                        {
                            infoBar.Show("Plugin failed to open the save file.");
                            return null;
                        }

                        var editor = new EditorViewModel(
                            doc,
                            sp.GetRequiredService<IBackupService>(),
                            sp.GetRequiredService<StatusBarViewModel>());
                        sp.GetRequiredService<MainViewModel>().OpenEditor(editor);
                        return editor;
                    }
                    catch (Exception ex)
                    {
                        infoBar.Show($"Failed to open save: {ex.Message}");
                        return null;
                    }
                }));
        services.AddSingleton<StoreViewModel>(sp =>
            new StoreViewModel(
                sp.GetRequiredService<ICatalogService>(),
                sp.GetRequiredService<IPluginInstaller>()));
        services.AddSingleton<SettingsViewModel>(sp =>
            new SettingsViewModel(
                settings,
                sp.GetRequiredService<SettingsService>(),
                sp.GetRequiredService<IUpdateChecker>(),
                sp.GetRequiredService<StatusBarViewModel>(),
                sp.GetRequiredService<InfoBarViewModel>()));
        services.AddSingleton<MainViewModel>();

        return services.BuildServiceProvider();
    }
}
