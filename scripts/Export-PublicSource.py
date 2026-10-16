"""Export a separate source-only publication draft; never push to a remote."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE_DIRS = (".cargo", "src", "crates", "tools", "scripts", "sheets", "re", "third_party")
PRUNE = {".git", "target", "artifacts", "build", "__pycache__", "_work",
         "_patchtree", "_patchrepo", "traces", "ppc", "wgsl", "wgpu", "out",
         "node_modules", "smoke-projects"}
EXTENSIONS = {".rs", ".toml", ".py", ".ps1", ".bat", ".cmd", ".sh", ".wgsl",
              ".c", ".cpp", ".h", ".hpp", ".java", ".csv", ".md", ".rules",
              ".patch", ".json", ".txt"}
TOP_FILES = ("Cargo.toml", "Cargo.lock", ".gitignore", "README.md")
SECRETS = re.compile(
    r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|"
    r"github_pat_[A-Za-z0-9_]{40,}|gh[pousr]_[A-Za-z0-9]{30,}|"
    r"AKIA[A-Z0-9]{16}|sk-proj-[A-Za-z0-9_-]{40,}"
)
UPSTREAM = "https://raw.githubusercontent.com/chasmlol/2010-rust-rewrite-mashup/f608f85/"


def source_files():
    selected = [ROOT / f for f in TOP_FILES]
    for folder in SOURCE_DIRS:
        for current, dirs, files in os.walk(ROOT / folder, followlinks=False):
            rel = Path(current).relative_to(ROOT).as_posix()
            dirs[:] = sorted(d for d in dirs if d not in PRUNE
                             and not d.endswith("-projects")
                             and not (Path(current) / d).is_symlink()
                             and not (rel == "re/capture" and d == "tools")
                             and not (rel == "crates/unleashed-guest" and d == "src"))
            for name in sorted(files):
                path = Path(current) / name
                if path.is_symlink() or name.startswith(".env"):
                    continue
                if rel == "re/oracle" and name == "stubs.cpp":
                    continue
                if rel == "crates/unleashed-guest" and name != "Cargo.toml":
                    continue
                if path.suffix.lower() in EXTENSIONS or name.startswith(("LICENSE", "NOTICE")):
                    selected.append(path)
    selected += [ROOT / "assets/resort-island" / f
                 for f in ("level.json", "LEVEL-README.md")]
    return sorted(set(selected))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path,
                        help="A new directory, outside the development checkout")
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists() or output == ROOT or ROOT in output.parents:
        raise SystemExit("Choose a new output folder outside the development checkout.")

    # Validate inputs before writing the draft. Never print any matching secret.
    sources = source_files()
    contents = {}
    for path in sources:
        text = path.read_text(encoding="utf-8-sig")
        if SECRETS.search(text):
            raise SystemExit(f"Potential credential in {path.relative_to(ROOT)}; export stopped.")
        text = text.replace("snapshots/82346F08_pre.bin",
                            "snapshots/82346F08_pre.bin")
        contents[path.relative_to(ROOT).as_posix()] = text

    # Explicit target path allows Cargo metadata to describe this crate before
    # local generation creates its ignored src/lib.rs.
    contents["crates/unleashed-guest/Cargo.toml"] += '\n[lib]\npath = "src/lib.rs"\n'
    contents[".gitignore"] += """
# Public source draft: supply these locally; do not stage game data or captures.
/artifacts/
/snapshots/
/game/
/update/
/refs/
/minecraft/
/crates/unleashed-guest/src/
/crates/unleashed-guest/lift_failures.json
/re/capture/_patchtree/
/re/capture/_patchrepo/
/re/ghidra/out/
/assets/resort-island/*
!/assets/resort-island/level.json
!/assets/resort-island/LEVEL-README.md
*.xex
*.xexp
*.bin
*.ar
*.pfd
*.iso
*.exe
*.dll
*.log
.env
.env.*
!.env.example
__pycache__/
"""
    for name in ("LICENSE", "NOTICE"):
        request = urllib.request.Request(UPSTREAM + name,
                                        headers={"User-Agent": "Unleashed-source-export"})
        with urllib.request.urlopen(request, timeout=30) as response:
            contents["third_party/" + name + "-mashup"] = response.read().decode("utf-8")
    contents["third_party/NOTICE.md"] += """

Export attribution update: LICENSE-mashup and NOTICE-mashup are unmodified copies
from chasmlol/2010-rust-rewrite-mashup at f608f85. Its NOTICE identifies the in-tree
MinecraftOSS code as its own author's project; these retain the upstream terms.
The local integration adapts workspace/path dependencies and uses the terrain
viewer implementation without its original GPU bindings. This source release
retains the existing local adaptations and does not imply a new project-wide license.
"""

    repo = output / "sonic-resort-island"
    repo.mkdir(parents=True)
    manifest = []
    for name, text in sorted(contents.items()):
        data = text.encode("utf-8")
        if len(data) >= 100 * 1024 * 1024:
            raise SystemExit(f"Oversized source file: {name}")
        dest = repo / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
        manifest.append({"path": name, "bytes": len(data),
                         "sha256": hashlib.sha256(data).hexdigest()})

    (output / "source-manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    archive = output / "sonic-resort-island-source.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zipped:
        for item in manifest:
            zipped.write(repo / item["path"], "sonic-resort-island/" + item["path"])
    summary = {"repository_folder": str(repo), "zip": str(archive),
               "files": len(manifest), "source_bytes": sum(f["bytes"] for f in manifest),
               "largest_file_bytes": max(f["bytes"] for f in manifest),
               "zip_bytes": archive.stat().st_size,
               "status": "Local source draft only; not published; clean build not validated"}
    (output / "export-summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
