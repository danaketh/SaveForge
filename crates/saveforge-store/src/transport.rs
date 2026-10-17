//! Reads `https://` and `file://` URLs with a size cap and progress reports.

use std::fs::File;
use std::io::{self, Read};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use saveforge_core::settings::path_from_file_url;
use saveforge_host::HOST_VERSION;
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::StoreError;

pub const MAX_CATALOG_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;

const CHUNK: usize = 64 * 1024;

static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(RootCerts::PlatformVerifier)
        .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .build();
    Agent::config_builder()
        .tls_config(tls)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        // A redirect must not downgrade to http://, where the catalog (and
        // with it every checksum) could be rewritten in transit.
        .https_only(true)
        .user_agent(format!("SaveForge/{}", *HOST_VERSION))
        .build()
        .new_agent()
});

/// The shared HTTP agent.
pub fn agent() -> &'static Agent {
    &AGENT
}

/// How long a body of up to `limit` bytes may take to arrive: a minute for
/// a catalog, ten for a package. A stalled connection then fails instead of
/// blocking the Store.
pub fn body_timeout(limit: u64) -> Duration {
    if limit <= MAX_CATALOG_BYTES {
        Duration::from_mins(1)
    } else {
        Duration::from_mins(10)
    }
}

/// Streams the body at `url` into `sink` and returns its length.
///
/// `progress(done, total)` is called after every chunk and once at the end;
/// `total` is the file length or the `Content-Length`, when known. More than
/// `limit` bytes fails with [`StoreError::TooLarge`].
pub fn read_url(
    url: &str,
    limit: u64,
    sink: &mut dyn io::Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64, StoreError> {
    if url.starts_with("file://") {
        let path =
            path_from_file_url(url).ok_or_else(|| StoreError::UnsupportedUrl(url.to_owned()))?;
        let mut file = File::open(&path).map_err(|e| StoreError::io(&path, e))?;
        let total = file.metadata().map(|m| m.len()).ok();
        copy(url, &mut file, total, limit, sink, progress, |e| {
            StoreError::io(&path, e)
        })
    } else if url.starts_with("https://") {
        let http = |message: String| StoreError::Http {
            url: url.to_owned(),
            message,
        };
        let mut response = AGENT
            .get(url)
            .config()
            .timeout_recv_body(Some(body_timeout(limit)))
            .build()
            .call()
            .map_err(|e| http(e.to_string()))?;
        let total = response.body().content_length();
        let mut reader = response.body_mut().as_reader();
        copy(url, &mut reader, total, limit, sink, progress, |e| {
            http(e.to_string())
        })
    } else {
        Err(StoreError::UnsupportedUrl(url.to_owned()))
    }
}

fn copy(
    url: &str,
    reader: &mut dyn Read,
    total: Option<u64>,
    limit: u64,
    sink: &mut dyn io::Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
    read_error: impl Fn(io::Error) -> StoreError,
) -> Result<u64, StoreError> {
    let too_large = || StoreError::TooLarge {
        url: url.to_owned(),
        limit,
    };
    if total.is_some_and(|t| t > limit) {
        return Err(too_large());
    }
    let mut buf = vec![0; CHUNK];
    let mut done = 0u64;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(read_error(e)),
        };
        done += n as u64;
        if done > limit {
            return Err(too_large());
        }
        sink.write_all(&buf[..n]).map_err(|e| StoreError::Http {
            url: url.to_owned(),
            message: format!("could not write the download: {e}"),
        })?;
        progress(done, total);
    }
    progress(done, total);
    Ok(done)
}
