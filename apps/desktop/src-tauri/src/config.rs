use crate::dto::{
    default_onboarding_completed_true, default_vessel_name, AppSettings, RootConfig,
};
use crate::path_norm;
use anyhow::Context;
use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const DEFAULT_EXCLUSIONS: &[&str] = &["**/node_modules/**", "**/.git/**", "**/target/**"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub roots: Vec<RootConfig>,
    #[serde(default)]
    pub exclusion_globs: Vec<String>,
    #[serde(default = "default_batch")]
    pub batch_size: usize,
    #[serde(default = "default_vessel_name")]
    pub vessel_name: String,
    #[serde(default = "default_onboarding_completed_true")]
    pub onboarding_completed: bool,
}

fn default_batch() -> usize {
    2000
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            roots: vec![],
            exclusion_globs: DEFAULT_EXCLUSIONS.iter().map(|s| (*s).to_string()).collect(),
            batch_size: 2000,
            vessel_name: default_vessel_name(),
            onboarding_completed: false,
        }
    }
}

pub fn load(path: &Path) -> anyhow::Result<AppSettings> {
    if !path.exists() {
        return Ok(ConfigFile::default().into());
    }
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let parsed: ConfigFile = serde_json::from_str(&raw).unwrap_or_default();
    let mut settings: AppSettings = parsed.into();
    if path_norm::normalize_app_settings(&mut settings) {
        info!(
            target: "vessel_paths",
            "normalized root paths in config; rewriting {}",
            path.display()
        );
        save(path, &settings)?;
    }
    Ok(settings)
}

pub fn save(path: &Path, settings: &AppSettings) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cfg = ConfigFile::from(settings.clone());
    let raw = serde_json::to_string_pretty(&cfg)?;
    fs::write(path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

impl From<ConfigFile> for AppSettings {
    fn from(c: ConfigFile) -> Self {
        let vessel_name = {
            let t = c.vessel_name.trim();
            if t.is_empty() {
                default_vessel_name()
            } else {
                t.to_string()
            }
        };
        AppSettings {
            roots: c.roots,
            exclusion_globs: if c.exclusion_globs.is_empty() {
                DEFAULT_EXCLUSIONS.iter().map(|s| (*s).to_string()).collect()
            } else {
                c.exclusion_globs
            },
            batch_size: c.batch_size,
            vessel_name,
            onboarding_completed: c.onboarding_completed,
        }
    }
}

impl From<AppSettings> for ConfigFile {
    fn from(s: AppSettings) -> Self {
        ConfigFile {
            roots: s.roots,
            exclusion_globs: s.exclusion_globs,
            batch_size: s.batch_size,
            vessel_name: s.vessel_name,
            onboarding_completed: s.onboarding_completed,
        }
    }
}
