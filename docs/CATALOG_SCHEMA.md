# Catalog Schema

The catalog is a JSON file served from the app's catalog URL. The app downloads it, caches it, and falls back to the cache when the download fails.

## Schema

```json
{
  "schemaVersion": "1.0",
  "generatedAt": "2026-04-04T00:00:00Z",
  "plugins": [
    {
      "id": "my-plugin",
      "name": "My Plugin",
      "description": "Short description shown in the store.",
      "author": "Author Name",
      "latestVersion": "1.2.0",
      "minHostVersion": "1.0.0",
      "maxHostVersion": null,
      "packageUrl": "https://example.com/releases/my-plugin-1.2.0.zip",
      "sha256": "lowercase hex SHA-256 of the zip file",
      "iconUrl": "https://example.com/icons/my-plugin.png",
      "screenshotUrls": [
        "https://example.com/screenshots/my-plugin-1.png"
      ],
      "tags": ["rpg", "jrpg"],
      "repositoryUrl": "https://github.com/example/my-plugin"
    }
  ]
}
```

## Field reference

| Field | Type | Required | Description |
|---|---|---|---|
| `schemaVersion` | string | Yes | Catalog format version (`"1.0"`) |
| `generatedAt` | string | No | When the catalog was generated |
| `plugins` | array | Yes | The entries below |
| `id` | string | Yes | Unique plugin identifier (lowercase, hyphens) |
| `name` | string | Yes | Display name |
| `description` | string | No | Short description (default empty) |
| `author` | string | Yes | Author name |
| `latestVersion` | string | Yes | SemVer string |
| `minHostVersion` | string | Yes | Minimum compatible host version |
| `maxHostVersion` | string/null | No | Maximum compatible host version, null = no limit |
| `packageUrl` | string | Yes | Direct download URL for the zip package |
| `sha256` | string | Yes | Hex SHA-256 of the zip |
| `iconUrl` | string/null | No | Icon image URL |
| `screenshotUrls` | string[] | No | Screenshot image URLs |
| `tags` | string[] | No | Search tags |
| `repositoryUrl` | string/null | No | Source repository URL |

## Rules the app enforces

- `id` must match `^[a-z0-9]+(-[a-z0-9]+)*$`.
- `latestVersion`, `minHostVersion` and `maxHostVersion` must be valid SemVer.
- `sha256` must be 64 hex digits, in either case.
- **One bad entry doesn't fail the catalog.** An entry with a missing required field or an invalid value is skipped and logged; the rest of the catalog still loads.
- **URL schemes:** the catalog URL and package URLs must be `https://` or `file://`. Redirects may not leave `https://`.
- **Remote catalogs:** an `https://` catalog may only list `https://` packages. Other entries are dropped. A `file://` URL that names another host or a network share is rejected.
- **Repository links:** `repositoryUrl` opens in the browser only when it is `https://`.
- **Size limits:** the catalog may be at most 8 MiB and a package at most 64 MiB. A package may expand to at most 256 MiB.
- **Entry status:** an entry is shown as *Incompatible* when the running app's version is outside `[minHostVersion, maxHostVersion]`. It is shown as *Update available* when an installed plugin is older than `latestVersion` and the new version is compatible.

## Package contents

A package is a zip file with these entries at its root:

```
manifest.json
plugin.wasm     (the manifest's "component" path)
icon.png        (optional, the manifest's "icon" path)
```

The manifest's `id` and `version` must equal the entry's `id` and `latestVersion`, or the install is refused. See [PLUGIN_AUTHORING.md](PLUGIN_AUTHORING.md) for the manifest format.

## Generating entries

```bash
cargo xtask package --base-url https://github.com/{owner}/{repo}/releases/download/v1.0.0/
```

This writes one zip per plugin and `catalog.json` to `target/packages/`, with each SHA-256 filled in. Without `--base-url`, the package URLs point at `target/packages/` itself (`file://`), which is handy for testing the Store locally. Zips are reproducible: the same plugin build gives the same SHA-256.

## Hosting

The release workflow (`.github/workflows/release.yml`) attaches the zips and `catalog.json` to each GitHub release. The newest catalog is then always at:

```
https://github.com/{owner}/{repo}/releases/latest/download/catalog.json
```

That is the release build's default catalog URL. You can change it in the app's Settings. Any `https://` URL serving the file works, for example a raw file in a repository.
