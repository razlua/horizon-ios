"""Which files of a prepared folder are Mojang's, byte for byte (the client jar's entries, or the
asset index's objects), and which Horizon generated (the Java exporters' tables and the rest).

    python -I classify_prepared.py PREPARED CLIENT_JAR ASSETS_DIR INDEX_ID OUT_JSON
"""
import hashlib
import json
import os
import sys
import zipfile

prepared, jar_path, assets, index_id, out = sys.argv[1:6]
SKIP_DIRS = {"classes", "client-classes"}

jar = zipfile.ZipFile(jar_path)
jar_entries = {i.filename: i for i in jar.infolist() if not i.filename.endswith("/")}
index = json.load(open(os.path.join(assets, "indexes", index_id + ".json"), encoding="utf-8"))
objects = index["objects"]  # name -> {hash, size}


def sha1(path):
    h = hashlib.sha1()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


kinds = {"jar": [], "object": [], "index": [], "horizon": []}
sizes = {k: 0 for k in kinds}
horizon_under_assets = []
for folder, dirs, files in os.walk(prepared):
    rel_dir = os.path.relpath(folder, prepared).replace(os.sep, "/")
    if rel_dir == ".":
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
    for name in files:
        path = os.path.join(folder, name)
        rel = os.path.relpath(path, prepared).replace(os.sep, "/")
        if name.endswith((".log", ".tmp")) or ".tmp" in name or name.startswith("."):
            continue
        if rel == "horizon-data.bin":
            continue
        size = os.path.getsize(path)
        kind = "horizon"
        entry = jar_entries.get(rel)
        if entry is not None and entry.file_size == size:
            if jar.read(rel) == open(path, "rb").read():
                kind = "jar"
        if kind == "horizon" and rel.startswith("assets/"):
            obj = objects.get(rel[len("assets/"):])
            if obj and obj["size"] == size and obj["hash"] == sha1(path):
                kind = "object"
        if kind == "horizon" and rel == "asset-index.json":
            if open(path, "rb").read() == open(os.path.join(assets, "indexes", index_id + ".json"), "rb").read():
                kind = "index"
        kinds[kind].append(rel)
        sizes[kind] += size
        if kind == "horizon" and (rel.startswith("assets/") or rel.startswith("data/")):
            horizon_under_assets.append(rel)

for k in kinds:
    print(f"{k:8} {len(kinds[k]):6} files {sizes[k] / 1e6:8.1f} MB")
print("horizon files under assets/ or data/:", len(horizon_under_assets))
for r in horizon_under_assets[:40]:
    print("  ", r)
tops = {}
for r in kinds["horizon"]:
    top = r.split("/")[0]
    tops[top] = tops.get(top, 0) + 1
print("horizon files by top level:", json.dumps(tops, indent=1))
json.dump(kinds, open(out, "w", encoding="utf-8"), indent=1)
