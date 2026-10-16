//! Minecraft's own files, fetched once from Mojang's official servers as the launcher
//! fetches them: the pinned 26.3 client JAR (textures, block models, world generation
//! data) and its asset index. Nothing of Mojang's ships with this project. Sounds are
//! left out unless `UNLEASHED_MINECRAFT_SOUNDS=1` (the controller does not use them).
//!
//! Adapted from 2010-rust-rewrite-mashup's `crates/assets/src/minecraft_setup.rs`
//! (Apache-2.0); the block, item and entity catalogs in `data/` come from there too.
//! The result is laid out as a MinecraftOSS checkout is, so `DataPaths::under` reads it.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

const VERSION: &str = "26.3";
/// The version's metadata, pinned by hash as Mojang's version manifest lists it.
const VERSION_URL: &str =
    "https://piston-meta.mojang.com/v1/packages/bc098d111a72e9f6178801544a42099bdfbb0cf2/26.3.json";
const VERSION_SHA1: &str = "bc098d111a72e9f6178801544a42099bdfbb0cf2";
const CLIENT_SHA1: &str = "e877b6a07acd633fb3bb475002175cec036e7b87";
const OBJECTS_URL: &str = "https://resources.download.minecraft.net";
const WORKERS: usize = 8;

pub const RESOURCE_PACK: &str = "resourcepacks/local/minecraft-26.3";
const DATA_PACK: &str = "datapacks/local/minecraft-26.3";
const CLIENT_JAR: &str = "client.jar";

const CATALOGS: [(&str, &[u8]); 4] = [
    ("artifacts/block-state-catalog/26.3.json", include_bytes!("../data/block-state-catalog-26.3.json.gz")),
    ("artifacts/block-entity-catalog/26.3.json", include_bytes!("../data/block-entity-catalog-26.3.json.gz")),
    ("artifacts/entity-catalog/26.3.json", include_bytes!("../data/entity-catalog-26.3.json.gz")),
    ("artifacts/item-catalog/26.3.json", include_bytes!("../data/item-catalog-26.3.json.gz")),
];

/// `<cache>/minecraft-26.3`.
pub fn dir_under(cache: &Path) -> PathBuf {
    cache.join(format!("minecraft-{VERSION}"))
}

/// The files under `cache`, fetching them first if they are not complete there.
pub fn ensure(cache: &Path) -> Result<PathBuf, String> {
    let dir = dir_under(cache);
    if dir.join(".complete").is_file() {
        return Ok(dir);
    }
    run(&dir)?;
    Ok(dir)
}

fn step(text: &str) {
    eprintln!("[minecraft] setup: {text}");
}

fn run(dir: &Path) -> Result<(), String> {
    let staging = dir.with_file_name(format!("minecraft-{VERSION}.partial"));
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;

    step("version metadata");
    let version_path = staging.join("version.json");
    fetch(VERSION_URL, &version_path, Some(VERSION_SHA1))?;
    let version = read_json(&version_path)?;
    let client_url = version["downloads"]["client"]["url"].as_str().ok_or("version has no client download")?;
    let index_url = version["assetIndex"]["url"].as_str().ok_or("version has no asset index")?;
    let index_sha1 = version["assetIndex"]["sha1"].as_str().ok_or("asset index has no hash")?;

    step("client JAR");
    let jar = staging.join(CLIENT_JAR);
    fetch(client_url, &jar, Some(CLIENT_SHA1))?;

    step("unpacking the client JAR");
    unpack_jar(&jar, &staging)?;

    step("asset index");
    let index_path = staging.join("asset-index.json");
    fetch(index_url, &index_path, Some(index_sha1))?;
    let index = read_json(&index_path)?;
    let objects = index["objects"].as_object().ok_or("asset index has no objects")?;
    let sounds = std::env::var("UNLEASHED_MINECRAFT_SOUNDS").is_ok_and(|v| v == "1");
    let assets = staging.join(RESOURCE_PACK).join("assets");
    let mut wanted = Vec::new();
    for (key, object) in objects {
        let keep = key == "minecraft/lang/en_us.json"
            || (sounds
                && (key == "minecraft/sounds.json"
                    || (key.starts_with("minecraft/sounds/")
                        && !key.starts_with("minecraft/sounds/music/")
                        && !key.starts_with("minecraft/sounds/records/"))));
        let Some(hash) = object["hash"].as_str().filter(|_| keep) else { continue };
        let path = assets.join(key);
        if sha1_of(&path).as_deref() != Some(hash) {
            wanted.push((hash.to_owned(), path));
        }
    }
    fetch_objects(&staging, &wanted)?;
    for (hash, path) in &wanted {
        if sha1_of(path).as_deref() != Some(hash.as_str()) {
            return Err(format!("{} did not match Mojang's hash", path.display()));
        }
    }

    step("catalogs");
    std::fs::write(
        staging.join(RESOURCE_PACK).join("pack.mcmeta"),
        r#"{"pack":{"description":"Minecraft 26.3, fetched from Mojang","min_format":[97,1],"max_format":[97,1]}}"#,
    )
    .map_err(|e| e.to_string())?;
    for (path, gz) in CATALOGS {
        let mut json = Vec::new();
        flate2::read::GzDecoder::new(gz).read_to_end(&mut json).map_err(|e| e.to_string())?;
        let out = staging.join(path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(out, json).map_err(|e| e.to_string())?;
    }

    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&staging, dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(".complete"), VERSION).map_err(|e| e.to_string())?;
    step("done");
    Ok(())
}

/// The JAR's `assets/` and pack icon into the resource pack, its `data/` into the data pack.
fn unpack_jar(jar: &Path, staging: &Path) -> Result<(), String> {
    let file = std::fs::File::open(jar).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name() else { continue };
        let out = if name.starts_with("assets") || name == Path::new("pack.png") {
            staging.join(RESOURCE_PACK).join(&name)
        } else if name.starts_with("data") {
            staging.join(DATA_PACK).join(&name)
        } else {
            continue;
        };
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The asset objects, split over a few `curl` processes each fetching its share.
fn fetch_objects(staging: &Path, wanted: &[(String, PathBuf)]) -> Result<(), String> {
    if wanted.is_empty() {
        return Ok(());
    }
    step(&format!("{} asset files", wanted.len()));
    let mut children = Vec::new();
    for (worker, share) in wanted.chunks(wanted.len().div_ceil(WORKERS)).enumerate() {
        let list = staging.join(format!("objects-{worker}.curl"));
        let mut text = String::new();
        for (hash, path) in share {
            let out = path.to_string_lossy().replace('\\', "/");
            text.push_str(&format!("url = \"{OBJECTS_URL}/{}/{hash}\"\noutput = \"{out}\"\n", &hash[..2]));
        }
        std::fs::File::create(&list).and_then(|mut f| f.write_all(text.as_bytes())).map_err(|e| e.to_string())?;
        let child = curl()
            .args(["--fail", "--silent", "--show-error", "--location", "--retry", "3", "--create-dirs", "--config"])
            .arg(&list)
            .spawn()
            .map_err(|e| format!("could not run curl: {e}"))?;
        children.push((child, list));
    }
    let mut failed = None;
    for (mut child, list) in children {
        let status = child.wait().map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(list);
        if !status.success() {
            failed = Some(format!("curl exited with {status}"));
        }
    }
    failed.map_or(Ok(()), Err)
}

/// One file, kept if it is already here with the right hash.
fn fetch(url: &str, path: &Path, sha1: Option<&str>) -> Result<(), String> {
    if sha1.is_some() && sha1_of(path).as_deref() == sha1 {
        return Ok(());
    }
    let status = curl()
        .args(["--fail", "--silent", "--show-error", "--location", "--retry", "3", "--create-dirs", "--output"])
        .arg(path)
        .arg(url)
        .status()
        .map_err(|e| format!("could not run curl: {e}"))?;
    if !status.success() {
        return Err(format!("downloading {url}: curl exited with {status}"));
    }
    match sha1 {
        Some(expected) if sha1_of(path).as_deref() != Some(expected) => Err(format!("{url} did not match its hash")),
        _ => Ok(()),
    }
}

fn curl() -> std::process::Command {
    let mut command = std::process::Command::new("curl");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // no console window
    }
    command
}

fn sha1_of(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(Sha1::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect())
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
