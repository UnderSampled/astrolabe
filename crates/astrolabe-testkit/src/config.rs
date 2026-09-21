//! Fixture validation for the already prepared private Hype installation.
use anyhow::{Context, Result, ensure};
use astrolabe::openspace::level::live::{HYPE_EXECUTABLE_SHA256, HypeBuild};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
pub const SAVE_SHA256: &str = "877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c";
const SAVE_MEMBERS: [(&str, &str); 4] = [
    (
        "General.sav",
        "af5570f5a1810b7af78caf4bc70a660f0df51e42baf91d4de5b2328de0e83dfc",
    ),
    (
        "General000.sav",
        "f4496eaa8e3b9a99e5bb450ef2265d4173a29035811c1f8a3288407e5b829653",
    ),
    (
        "Level000.sav",
        "6786fe4f781b04d1c674e33f04f40b95c7ec9f1f82e0cff5d56574b1cddb672f",
    ),
    (
        "manoir.sav",
        "d87d6b743514e070983633385742ab16c6f8a189ad6a01ff80e5bb636c7df6c7",
    ),
];
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub prefix: PathBuf,
    pub wine: PathBuf,
    pub game: PathBuf,
    pub exe: PathBuf,
    pub exe_sha256: String,
    pub save_sha256: String,
}
pub fn digest(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 65536];
    loop {
        let size = file.read(&mut chunk)?;
        if size == 0 {
            break;
        }
        hash.update(&chunk[..size]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let config: Self = serde_json::from_slice(&fs::read(path)?)?;
        let directory = path
            .canonicalize()?
            .parent()
            .context("configuration parent missing")?
            .to_owned();
        ensure!(
            config.prefix.canonicalize()? == directory.join("prefix").canonicalize()?,
            "refusing an installation outside the prepared private prefix"
        );
        ensure!(
            config
                .game
                .canonicalize()?
                .starts_with(config.prefix.canonicalize()?)
                && config.exe.canonicalize()?.parent()
                    == Some(config.game.canonicalize()?.as_path()),
            "game executable is outside the private installation"
        );
        ensure!(config.wine.is_file(), "configured Wine runner is missing");
        ensure!(
            config.exe_sha256 == HYPE_EXECUTABLE_SHA256
                && digest(&config.exe)? == config.exe_sha256,
            "executable fixture hash differs"
        );
        HypeBuild::verified(&config.exe_sha256)?;
        ensure!(
            config.save_sha256 == SAVE_SHA256
                && digest(&directory.join("Savefile_EN.zip"))? == SAVE_SHA256,
            "English save archive differs"
        );
        for (name, expected) in SAVE_MEMBERS {
            ensure!(
                digest(&config.game.join("GameData/SaveGame/SaveGame0").join(name))? == expected,
                "installed save fixture differs: {name}"
            );
        }
        Ok(config)
    }
    pub fn environment(&self) -> Result<std::collections::BTreeMap<String, String>> {
        let runner = self
            .wine
            .parent()
            .and_then(Path::parent)
            .context("Wine runner path")?;
        let user = std::env::var_os("HOME").context("HOME missing")?;
        let runtime = PathBuf::from(user).join(".local/share/lutris/runtime");
        let mut libraries = vec![
            self.prefix.join("hype-runtime/lib32"),
            runner.join("lib"),
            runner.join("lib64"),
            PathBuf::from("/usr/lib"),
            PathBuf::from("/usr/lib32"),
        ];
        for (arch, debarch) in [("i386", "i386"), ("amd64", "x86_64")] {
            for suffix in [
                format!("lib/{debarch}-linux-gnu"),
                "lib".into(),
                format!("usr/lib/{debarch}-linux-gnu"),
                "usr/lib".into(),
            ] {
                libraries.push(runtime.join("steam").join(arch).join(suffix));
            }
        }
        libraries.extend([
            runtime.join("Ubuntu-18.04-i686"),
            runtime.join("Ubuntu-18.04-x86_64"),
        ]);
        let library_path = libraries
            .iter()
            .filter(|p| p.is_dir())
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join(":");
        Ok(std::collections::BTreeMap::from([
            ("WINEPREFIX".into(), self.prefix.display().to_string()),
            ("WINEARCH".into(), "win32".into()),
            ("WINEDEBUG".into(), "-all".into()),
            ("WINEESYNC".into(), "1".into()),
            ("WINEFSYNC".into(), "1".into()),
            (
                "WINEDLLOVERRIDES".into(),
                "winemenubuilder=;ddraw,d3dimm,glide2x,glide3x,d3d11,dxgi=n,b".into(),
            ),
            ("LD_LIBRARY_PATH".into(), library_path),
            (
                "GST_PLUGIN_SYSTEM_PATH_1_0".into(),
                runner.join("lib/gstreamer-1.0").display().to_string(),
            ),
            ("__NV_PRIME_RENDER_OFFLOAD".into(), "1".into()),
            ("__GLX_VENDOR_LIBRARY_NAME".into(), "nvidia".into()),
            ("__VK_LAYER_NV_optimus".into(), "NVIDIA_only".into()),
        ]))
    }
}
