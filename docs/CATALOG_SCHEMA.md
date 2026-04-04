# Catalog Schema

The catalog is a JSON file served from a configured URL. The app downloads and caches it.

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

## Field Reference

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | Yes | Unique plugin identifier (lowercase, hyphens) |
| `name` | string | Yes | Display name |
| `description` | string | No | Short description |
| `author` | string | Yes | Author name |
| `latestVersion` | string | Yes | SemVer string |
| `minHostVersion` | string | Yes | Minimum compatible host version |
| `maxHostVersion` | string/null | No | Maximum compatible host version, null = no limit |
| `packageUrl` | string | Yes | Direct download URL for the zip package |
| `sha256` | string | Yes | Lowercase hex SHA-256 of the zip |
| `iconUrl` | string/null | No | Icon image URL |
| `screenshotUrls` | string[] | No | Screenshot image URLs |
| `tags` | string[] | No | Search tags |
| `repositoryUrl` | string/null | No | Source repository URL |

## Hosting on GitHub

Place `catalog.json` in a public GitHub repo. The raw URL format is:

```
https://raw.githubusercontent.com/{owner}/{repo}/{branch}/catalog.json
```

Set this URL in the app's Settings page or in `appsettings.json` for the release build.
