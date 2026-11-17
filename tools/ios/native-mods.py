"""The iOS all-mods build: Horizon's registry native mods listed in tools/ios/native-mods.txt,
applied to this source with horizon-mixins as the launcher's native build does
(launcher/src/native_mods.rs: each bundle repinned to the build's revision, then `apply`). They
start off in the app (mod_list::NATIVE_STARTS_OFF); each player turns on the ones they want.

Usage:
  python tools/ios/native-mods.py fetch <work dir>
      Downloads each listed .hnmod, checks its SHA-256 and unpacks it.
  python tools/ios/native-mods.py apply <work dir> <horizon-mixins> <out dir>
      Applies the unpacked mods to this checkout, into <out dir> (a full, modded source tree).
  python tools/ios/native-mods.py notices <work dir>
      Writes NATIVE_MODS.md: each mod, its version, license and source.
"""

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.request
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LIST = os.path.join(ROOT, "tools", "ios", "native-mods.txt")
# The upstream commit this port is built on: every mod is repinned to it.
REVISION = "6475f62f3c834b7489e1816bf47f47ea5eb2b879"


def listed():
    """(id, version, sha256, url, license, source) for each line of the list."""
    mods = []
    for line in open(LIST, encoding="utf-8"):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        mod_id, version, sha, url, license_, source = line.split("\t")
        mods.append((mod_id, version, sha, url, license_, source))
    return mods


def fetch(work):
    os.makedirs(os.path.join(work, "files"), exist_ok=True)
    for mod_id, version, sha, url, _, _ in listed():
        path = os.path.join(work, "files", f"{mod_id}-{version}.hnmod")
        if not os.path.isfile(path):
            req = urllib.request.Request(url, headers={"User-Agent": "horizon-ios-build"})
            with urllib.request.urlopen(req, timeout=120) as r:
                data = r.read()
            with open(path, "wb") as f:
                f.write(data)
        data = open(path, "rb").read()
        got = hashlib.sha256(data).hexdigest()
        if got != sha:
            sys.exit(f"{mod_id} {version}: SHA-256 {got}, the list says {sha}")
        out = os.path.join(work, "unpacked", mod_id)
        shutil.rmtree(out, ignore_errors=True)
        with zipfile.ZipFile(path) as z:
            for name in z.namelist():
                # No absolute paths or `..` out of the folder.
                if name.startswith(("/", "\\")) or ".." in name.replace("\\", "/").split("/"):
                    sys.exit(f"{mod_id}: unsafe path in the archive: {name}")
            z.extractall(out)
    print(f"{len(listed())} mods fetched and checked")


def apply(work, tool, out):
    inp = os.path.join(work, "mixins-in")
    shutil.rmtree(inp, ignore_errors=True)
    os.makedirs(inp)
    for mod_id, *_ in listed():
        mixins = os.path.join(work, "unpacked", mod_id, "mixins")
        for bundle in sorted(os.listdir(mixins)):
            src = os.path.join(mixins, bundle)
            if not os.path.isfile(os.path.join(src, "horizon-mixin.toml")):
                continue
            dest = os.path.join(inp, bundle)
            shutil.copytree(src, dest)
            manifest = os.path.join(dest, "horizon-mixin.toml")
            text = open(manifest, encoding="utf-8").read()
            text = re.sub(r'(?m)^horizon_revision\s*=\s*"[0-9a-f]+"',
                          f'horizon_revision = "{REVISION}"', text)
            open(manifest, "w", encoding="utf-8", newline="\n").write(text)
    # horizon-mixins reads the workspace's revision from here, as in the launcher's source copy.
    with open(os.path.join(ROOT, ".horizon-revision"), "w") as f:
        f.write(REVISION + "\n")
    shutil.rmtree(out, ignore_errors=True)
    r = subprocess.run([tool, "apply", "--workspace", ROOT, "--mods", inp, "--out", out])
    sys.exit(r.returncode)


def notices(work):
    lines = [
        "# Native mods in this build",
        "",
        "The iOS app is built with these mods from [Horizon's registry](https://github.com/horizon-rust/horizon-mods).",
        "They start off: turn on the ones you want in *Options → Mods*. Each belongs to its authors, under its own license.",
        "",
        "| Mod | Version | License | Source |",
        "|---|---|---|---|",
    ]
    for mod_id, version, _, _, license_, source in listed():
        lines.append(f"| `{mod_id}` | {version} | {license_} | {source} |")
    with open(os.path.join(ROOT, "NATIVE_MODS.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def main():
    args = sys.argv[1:]
    if args[:1] == ["fetch"] and len(args) == 2:
        fetch(args[1])
    elif args[:1] == ["apply"] and len(args) == 4:
        apply(args[1], args[2], args[3])
    elif args[:1] == ["notices"] and len(args) == 2:
        notices(args[1])
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
