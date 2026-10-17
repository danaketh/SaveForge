//! The Settings screen's editable copy of [`AppSettings`] (spec §5.2 Settings).

use std::path::PathBuf;

use saveforge_core::settings::AppSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    CatalogUrl,
    PluginDirectory,
    BackupDirectory,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsForm {
    pub catalog_url: String,
    pub plugin_directory: String,
    pub backup_directory: String,
    pub check_updates_on_startup: bool,
    /// From the last failed save, shown under the form.
    pub errors: Vec<String>,
}

impl SettingsForm {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            catalog_url: settings.catalog_url.clone(),
            plugin_directory: settings.plugin_directory.display().to_string(),
            backup_directory: settings.backup_directory.display().to_string(),
            check_updates_on_startup: settings.check_updates_on_startup,
            errors: Vec::new(),
        }
    }

    pub fn field_mut(&mut self, field: SettingsField) -> &mut String {
        match field {
            SettingsField::CatalogUrl => &mut self.catalog_url,
            SettingsField::PluginDirectory => &mut self.plugin_directory,
            SettingsField::BackupDirectory => &mut self.backup_directory,
        }
    }

    /// Problems with the form, in field order; empty when it can be saved.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let url = self.catalog_url.trim();
        if !(url.starts_with("https://") || url.starts_with("file://")) {
            errors.push("Catalog URL must start with https:// or file://".to_owned());
        }
        for (label, value) in [
            ("Plugin directory", &self.plugin_directory),
            ("Backup directory", &self.backup_directory),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{label} must not be empty."));
            }
        }
        errors
    }

    /// Copies the form into `settings`, leaving the per-plugin save folders
    /// alone. Returns whether the plugin directory changed.
    pub fn apply(&self, settings: &mut AppSettings) -> bool {
        let plugin_directory = PathBuf::from(self.plugin_directory.trim());
        let changed = plugin_directory != settings.plugin_directory;
        self.catalog_url
            .trim()
            .clone_into(&mut settings.catalog_url);
        settings.plugin_directory = plugin_directory;
        settings.backup_directory = PathBuf::from(self.backup_directory.trim());
        settings.check_updates_on_startup = self.check_updates_on_startup;
        changed
    }
}
