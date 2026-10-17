//! Package fixtures for the install tests.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::io::{Cursor, Write};
use std::path::Path;

use saveforge_core::settings::file_url;
use saveforge_store::CatalogEntry;
use semver::Version;
use sha2::{Digest, Sha256};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

/// A zip holding `files` (name, bytes), stored uncompressed.
pub fn package(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in files {
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

/// A valid manifest for `id` at `version` whose component is `plugin.wasm`.
pub fn manifest_json(id: &str, version: &str, min_host: &str) -> String {
    format!(
        r#"{{"id":"{id}","name":"Demo Plugin","version":"{version}","author":"Me",
"apiVersion":"0.1","minHostVersion":"{min_host}","component":"plugin.wasm",
"fileExtensions":[".demo"]}}"#
    )
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, b| {
            write!(out, "{b:02x}").unwrap();
            out
        })
}

pub fn entry_for(id: &str, version: &str, package_path: &Path, sha: &str) -> CatalogEntry {
    CatalogEntry {
        id: id.to_owned(),
        name: "Demo Plugin".to_owned(),
        description: String::new(),
        author: "Me".to_owned(),
        latest_version: Version::parse(version).unwrap(),
        min_host_version: Version::new(0, 1, 0),
        max_host_version: None,
        package_url: file_url(package_path),
        sha256: sha.to_owned(),
        icon_url: None,
        screenshot_urls: Vec::new(),
        tags: Vec::new(),
        repository_url: None,
    }
}
