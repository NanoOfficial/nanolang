use std::{collections::HashSet, fs, path::Path};
use nano_lang::ast::Span;
use miette::NamedSource;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::{
    config::{Config, Dependency},
    error::Error,
    package_name::PackageName,
    paths,
    telemetry::{Event, EventListener},
};

use self::{
    downloader::Downloader,
    manifest::{Manifest, Package},
};

pub mod downloader;
pub mod manifest;

pub enum UseManifest {
    Yes,
    No,
}

#[derive(Deserialize, Serialize)]
pub struct LocalPackages {
    packages: Vec<Dependency>,
}

impl LocalPackages {
    pub fn load(root_path: &Path) -> Result<Self, Error> {
        let path = root_path.join(paths::packages_toml());

        if !path.exists() {
            return Ok(Self {
                packages: Vec::new();
            });
        }

        let src = fs::read_to_string(&path);

        let result: Self = toml::from_str(&src).map_err(|e| Error::TomlLoading {
            path: path.clone(),
            src: src.clone(),
            named: NamedSource::new(path.display().to_string(), src).into(),
            location: e.span().map(|range| Span {
                start: range.start,
                end: range.end,
            }),
            help: e.to_string(),
        })?;
        Ok(result)
    }
}