use saveforge_store::{Catalog, EntryStatus, StoreError, Update};
use semver::Version;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const CATALOG: &str = r#"{
  "schemaVersion": "1",
  "generatedAt": "2026-10-17T00:00:00Z",
  "plugins": [
    {
      "id": "planet-crafter",
      "name": "Planet Crafter",
      "description": "Edit your terraforming progress.",
      "author": "SaveForge Team",
      "latestVersion": "1.2.0",
      "minHostVersion": "0.1.0",
      "packageUrl": "https://example.com/planet-crafter-1.2.0.zip",
      "sha256": "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF",
      "tags": ["survival"]
    },
    {
      "id": "stardew-like",
      "name": "StardewLike",
      "author": "Someone",
      "latestVersion": "0.1.0",
      "minHostVersion": "0.1.0",
      "maxHostVersion": "0.1.5",
      "packageUrl": "https://example.com/stardew-like-0.1.0.zip",
      "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    },
    {
      "id": "future-only",
      "name": "Future",
      "author": "Someone",
      "latestVersion": "2.0.0",
      "minHostVersion": "9.0.0",
      "packageUrl": "https://example.com/future-only-2.0.0.zip",
      "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }
  ]
}"#;

fn host() -> Version {
    Version::new(0, 1, 0)
}

fn v(s: &str) -> Version {
    Version::parse(s).unwrap()
}

fn catalog() -> Catalog {
    Catalog::parse(CATALOG).unwrap()
}

fn ids(entries: &[&saveforge_store::CatalogEntry]) -> Vec<String> {
    entries.iter().map(|e| e.id.clone()).collect()
}

#[test]
fn parses_entries_in_order() {
    let catalog = catalog();
    assert_eq!(catalog.schema_version, "1");
    assert_eq!(
        catalog.generated_at.as_deref(),
        Some("2026-10-17T00:00:00Z")
    );
    let ids: Vec<&str> = catalog.plugins.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["planet-crafter", "stardew-like", "future-only"]);
    let planet = catalog.entry("planet-crafter").unwrap();
    assert_eq!(planet.sha256, SHA);
    assert_eq!(planet.tags, ["survival"]);
    assert_eq!(catalog.entry("stardew-like").unwrap().description, "");
    assert_eq!(
        catalog.entry("stardew-like").unwrap().max_host_version,
        Some(v("0.1.5"))
    );
    assert_eq!(catalog.entry("nope"), None);
}

#[test]
fn malformed_entries_are_skipped() {
    let mut value: serde_json::Value = serde_json::from_str(CATALOG).unwrap();
    let plugins = value["plugins"].as_array_mut().unwrap();
    let good = plugins[1].clone();
    let mut bad_version = good.clone();
    bad_version["id"] = "bad-version".into();
    bad_version["latestVersion"] = "one".into();
    let mut no_url = good.clone();
    no_url["id"] = "no-url".into();
    no_url.as_object_mut().unwrap().remove("packageUrl");
    let mut bad_id = good.clone();
    bad_id["id"] = "../x".into();
    let mut bad_sha = good;
    bad_sha["id"] = "bad-sha".into();
    bad_sha["sha256"] = "abc".into();
    plugins.insert(1, bad_version);
    plugins.push(no_url);
    plugins.push(bad_id);
    plugins.push(bad_sha);
    plugins.push(serde_json::json!(42));

    let catalog = Catalog::parse(&value.to_string()).unwrap();
    let ids: Vec<&str> = catalog.plugins.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["planet-crafter", "stardew-like", "future-only"]);
}

#[test]
fn non_catalog_json_is_rejected() {
    for text in [
        "[]",
        r#"{"plugins":1}"#,
        r#"{"schemaVersion":"1","plugins":{}}"#,
    ] {
        assert!(
            matches!(Catalog::parse(text), Err(StoreError::InvalidCatalog(_))),
            "{text}"
        );
    }
    assert!(Catalog::parse("nope").is_err());
}

#[test]
fn compatibility_uses_min_and_max() {
    let catalog = catalog();
    assert!(
        catalog
            .entry("planet-crafter")
            .unwrap()
            .is_compatible(&host())
    );
    let stardew = catalog.entry("stardew-like").unwrap();
    assert!(stardew.is_compatible(&host()));
    assert!(stardew.is_compatible(&v("0.1.5")));
    assert!(!stardew.is_compatible(&v("0.2.0")));
    assert!(!catalog.entry("future-only").unwrap().is_compatible(&host()));
}

#[test]
fn status_covers_every_case() {
    let catalog = catalog();
    let planet = catalog.entry("planet-crafter").unwrap();
    let future = catalog.entry("future-only").unwrap();
    assert_eq!(future.status(None, &host()), EntryStatus::Incompatible);
    assert_eq!(planet.status(None, &host()), EntryStatus::NotInstalled);
    assert_eq!(
        planet.status(Some(&v("1.0.0")), &host()),
        EntryStatus::UpdateAvailable {
            installed: v("1.0.0")
        }
    );
    assert_eq!(
        planet.status(Some(&v("1.2.0")), &host()),
        EntryStatus::Installed {
            version: v("1.2.0")
        }
    );
    assert_eq!(
        future.status(Some(&v("1.0.0")), &host()),
        EntryStatus::Installed {
            version: v("1.0.0")
        }
    );
}

#[test]
fn search_matches_name_description_author_and_tags() {
    let catalog = catalog();
    assert_eq!(ids(&catalog.search("PLANET")), ["planet-crafter"]);
    assert_eq!(ids(&catalog.search("terraforming")), ["planet-crafter"]);
    assert_eq!(ids(&catalog.search("survival")), ["planet-crafter"]);
    assert_eq!(ids(&catalog.search("saveforge team")), ["planet-crafter"]);
    assert_eq!(
        ids(&catalog.search("  ")),
        ["planet-crafter", "stardew-like", "future-only"]
    );
    assert_eq!(ids(&catalog.search("zzz")), Vec::<String>::new());
}

#[test]
fn updates_lists_only_compatible_newer_versions() {
    let installed = [
        ("planet-crafter".to_owned(), v("1.0.0")),
        ("future-only".to_owned(), v("1.0.0")),
        ("gone".to_owned(), v("1.0.0")),
    ];
    assert_eq!(
        catalog().updates(&installed, &host()),
        [Update {
            id: "planet-crafter".into(),
            name: "Planet Crafter".into(),
            installed: v("1.0.0"),
            latest: v("1.2.0"),
        }]
    );
}
