//! Filesystem location helpers shared by every front-end.

use std::path::PathBuf;

/// Application data directory (`<data_local>/ripdown`), used for cached binaries.
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ripdown")
}

/// Directory holding the auto-downloaded yt-dlp + ffmpeg binaries.
pub fn libs_dir() -> PathBuf {
    data_dir().join("libs")
}

/// Resolve the output directory: explicit arg → `RIPDOWN_OUTPUT_DIR` → `~/Downloads/ripdown`.
pub fn resolve_output_dir(arg: Option<PathBuf>) -> PathBuf {
    arg.or_else(|| std::env::var("RIPDOWN_OUTPUT_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| {
            dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("ripdown")
        })
}

/// Whether the yt-dlp + ffmpeg binaries have already been downloaded.
pub fn binaries_present() -> bool {
    let libs = libs_dir();
    libs.join("yt-dlp").exists() && libs.join("ffmpeg").exists()
}

// ---------------------------------------------------------------------------
// Libs manifest — detects binaries installed by an older ripdown version
// ---------------------------------------------------------------------------

/// Version of the running ripdown build (workspace version).
pub const RIPDOWN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default lifetime of the cached binaries before a refresh is suggested.
/// yt-dlp breaks whenever YouTube changes, so stale binaries are a real bug
/// source. Override with `RIPDOWN_LIBS_MAX_AGE_DAYS` (`0` disables the check).
const DEFAULT_LIBS_MAX_AGE_DAYS: i64 = 30;

/// Stamp written next to the binaries recording which ripdown installed them.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibsManifest {
    /// ripdown version that installed these binaries.
    pub ripdown_version: String,
    /// UTC timestamp of the install.
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

/// Path of the manifest file inside [`libs_dir`].
pub fn libs_manifest_path() -> PathBuf {
    libs_dir().join(".ripdown-libs.json")
}

/// Read the manifest, if the binaries were installed by a version that wrote one.
pub fn read_libs_manifest() -> Option<LibsManifest> {
    let raw = std::fs::read_to_string(libs_manifest_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Record the running version as the owner of the currently installed binaries.
pub fn write_libs_manifest() -> std::io::Result<()> {
    let manifest = LibsManifest {
        ripdown_version: RIPDOWN_VERSION.to_string(),
        installed_at: chrono::Utc::now(),
    };
    std::fs::create_dir_all(libs_dir())?;
    let json = serde_json::to_string_pretty(&manifest).map_err(std::io::Error::other)?;
    std::fs::write(libs_manifest_path(), json)
}

/// Age limit (in days) before cached binaries count as stale; `None` disables it.
fn libs_max_age_days() -> Option<i64> {
    match std::env::var("RIPDOWN_LIBS_MAX_AGE_DAYS") {
        Ok(v) => match v.trim().parse::<i64>() {
            Ok(0) => None,
            Ok(d) if d > 0 => Some(d),
            _ => Some(DEFAULT_LIBS_MAX_AGE_DAYS),
        },
        Err(_) => Some(DEFAULT_LIBS_MAX_AGE_DAYS),
    }
}

/// Why the cached binaries should be reinstalled, if they should.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibsState {
    /// Nothing installed yet — a fresh install is required.
    Missing,
    /// Installed, stamped with the running version and recent enough.
    Current,
    /// Installed by an older/different ripdown, or with no manifest at all
    /// (pre-0.3.x layouts), or older than the max-age window.
    Stale(String),
}

/// Classify the binaries cached in [`libs_dir`].
pub fn libs_state() -> LibsState {
    if !binaries_present() {
        return LibsState::Missing;
    }

    // Escape hatch for pre-baked images (Docker), where the binaries are
    // installed by the build and must never be re-fetched at runtime.
    if std::env::var("RIPDOWN_SKIP_LIB_UPDATE").is_ok_and(|v| v != "0" && !v.is_empty()) {
        return LibsState::Current;
    }

    let Some(manifest) = read_libs_manifest() else {
        return LibsState::Stale("installed by an older ripdown (no version stamp)".into());
    };

    if manifest.ripdown_version != RIPDOWN_VERSION {
        return LibsState::Stale(format!(
            "installed by ripdown {} (running {RIPDOWN_VERSION})",
            manifest.ripdown_version
        ));
    }

    if let Some(max_age) = libs_max_age_days() {
        let age = (chrono::Utc::now() - manifest.installed_at).num_days();
        if age > max_age {
            return LibsState::Stale(format!("installed {age} days ago (limit {max_age})"));
        }
    }

    LibsState::Current
}

/// Whether the cached binaries were left behind by an older ripdown / are aged out.
pub fn libs_stale() -> bool {
    matches!(libs_state(), LibsState::Stale(_))
}

/// Delete the cached binaries so the next run reinstalls them from scratch.
pub fn clear_libs() -> std::io::Result<()> {
    let libs = libs_dir();
    if libs.exists() {
        std::fs::remove_dir_all(&libs)?;
    }
    Ok(())
}
