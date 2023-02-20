use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::bail;
use bytes::buf::Buf;
use sha2::Digest;

#[cfg(target_os = "windows")]
const TAILWIND_SHA256: &str = "e7f8a638629ffd39432969ccbd1825131e1c9ad5afc3b296b615ec37a4c4db37";
#[cfg(target_os = "windows")]
const TAILWIND_URL: &str = "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.2.7/tailwindcss-windows-x64.exe";

#[cfg(target_os = "macos")]
const TAILWIND_SHA256: &str = "586ed430def4b54cc6a5b326cfb7315cd7ac772f7ecc6ddd99b9ce09d6cb3de1";
#[cfg(target_os = "macos")]
const TAILWIND_URL: &str =
    "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.2.7/tailwindcss-macos-x64";

#[cfg(target_os = "linux")]
const TAILWIND_SHA256: &str = "35e4fa253af4ddab73490b7443b7d08f0c664a8d8b3b878eadcbb54a7e0647f8";
#[cfg(target_os = "linux")]
const TAILWIND_URL: &str =
    "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.2.7/tailwindcss-linux-x64";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let staging_dir = std::env::var("TRUNK_STAGING_DIR").map(PathBuf::from).expect(
        "TRUNK_STAGING_DIR needs to be set. This script is intended to be run by the trunk command.",
    );
    let tailwind = setup(&staging_dir).await?;
    build(&staging_dir, &tailwind)?;
    Ok(())
}

async fn setup(staging_dir: &Path) -> anyhow::Result<PathBuf> {
    let path = staging_dir.join("tailwindcss");
    if path.exists() {
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = sha2::Sha256::new();
        std::io::copy(&mut reader, &mut hasher)?;
        let hash = hasher.finalize();
        if TAILWIND_SHA256 == &format!("{:x}", hash) {
            return Ok(path);
        }
    }
    let file = File::create(&path)?;
    let mut wtr = BufWriter::new(file);
    let bytes = reqwest::get(TAILWIND_URL).await?.bytes().await?;
    std::io::copy(&mut bytes.reader(), &mut wtr)?;
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, PermissionsExt::from_mode(0x755))?;
    }
    Ok(path)
}

fn build(staging_dir: &Path, tailwind: &Path) -> anyhow::Result<()> {
    let status = Command::new(tailwind)
        .args([
            "build",
            "-i",
            "src/tailwind.css",
            "-o",
            &format!("{}", staging_dir.join("tailwind.css").display()),
        ])
        .env("TRUNK_STAGING_DIR", staging_dir)
        .status()?;
    if !status.success() {
        bail!("{status}");
    }
    Ok(())
}
