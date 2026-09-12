use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageRole {
    Fix,
    Level,
    Texture,
    Sound,
}

/// Filesystem content identity, with an optional named member. No VM address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssetPath(String);

impl AssetPath {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let (role, path) = value
            .split_once(":/")
            .ok_or_else(|| anyhow::anyhow!("asset reference needs role:/path: {value}"))?;
        ensure!(
            matches!(role, "fix" | "level" | "texture" | "sound"),
            "unknown package role: {role}"
        );
        let (file, member) = path
            .split_once('#')
            .map_or((path, None), |(f, m)| (f, Some(m)));
        ensure!(
            !file.is_empty() && !file.contains(['\\', ':', '\0']),
            "invalid asset path: {value}"
        );
        ensure!(
            file.split('/')
                .all(|s| !s.is_empty() && s != "." && s != ".."),
            "asset path escapes package: {value}"
        );
        ensure!(
            member.is_none_or(|s| !s.is_empty() && !s.contains(['#', '\0'])),
            "invalid member: {value}"
        );
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn role(&self) -> PackageRole {
        match self.0.split_once(":/").unwrap().0 {
            "fix" => PackageRole::Fix,
            "level" => PackageRole::Level,
            "texture" => PackageRole::Texture,
            _ => PackageRole::Sound,
        }
    }
    pub fn file(&self) -> &str {
        self.0
            .split_once(":/")
            .unwrap()
            .1
            .split('#')
            .next()
            .unwrap()
    }
    pub fn member(&self) -> Option<&str> {
        self.0.split_once('#').map(|(_, m)| m)
    }
}
impl TryFrom<String> for AssetPath {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}
impl From<AssetPath> for String {
    fn from(path: AssetPath) -> String {
        path.0
    }
}
impl std::fmt::Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent, bound = "")]
pub struct AssetRef<T> {
    pub path: AssetPath,
    #[serde(skip)]
    marker: PhantomData<fn() -> T>,
}
impl<T> AssetRef<T> {
    pub fn new(path: impl Into<String>) -> Result<Self> {
        Ok(Self {
            path: AssetPath::new(path)?,
            marker: PhantomData,
        })
    }
}

/// Each import/export session binds one selected level to the shared packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub level: String,
    pub locale: String,
    pub profile: String,
}
impl Selection {
    pub fn hype(level: impl Into<String>, locale: impl Into<String>) -> Result<Self> {
        let value = Self {
            level: level.into(),
            locale: locale.into(),
            profile: "hype-pc-montreal".into(),
        };
        ensure!(
            !value.level.is_empty()
                && !value.level.contains(['/', '\\', ':', '\0'])
                && value.level != "."
                && value.level != "..",
            "invalid level selection"
        );
        ensure!(
            !value.locale.is_empty()
                && value.locale != "."
                && value.locale != ".."
                && !value.locale.contains(['/', '\\', ':', '\0']),
            "invalid locale selection"
        );
        Ok(value)
    }
}
