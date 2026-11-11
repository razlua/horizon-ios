"""The tables bundle for the on-device setup: the files Horizon's preparation generates (the Java
exporters' tables), none of Mojang's, and `ios-setup.json`, which names what the phone downloads
from Mojang itself (the client jar's entries and the asset index's objects, with their hashes).

    python -I make_tables.py PREPARED CLASSIFIED_JSON VERSION_JSON ASSETS_DIR OUT_ZIP
"""
import hashlib
import json
import os
import sys
import zipfile

prepared, classified, version_json, assets, out = sys.argv[1:6]
kinds = json.load(open(classified, encoding="utf-8"))
version = json.load(open(version_json, encoding="utf-8"))
index_id = version["assetIndex"]["id"]
index = json.load(open(os.path.join(assets, "indexes", index_id + ".json"), encoding="utf-8"))
manifest = json.load(open(os.path.join(prepared, "horizon-resources.json"), encoding="utf-8"))

setup = {
    "format": 1,
    "minecraft": manifest["minecraft"],
    "client": version["downloads"]["client"],
    "asset_index": version["assetIndex"],
    "jar_files": sorted(kinds["jar"]),
    "objects": [
        {"name": n, "hash": index["objects"][n]["hash"], "size": index["objects"][n]["size"]}
        for n in sorted(p[len("assets/"):] for p in kinds["object"])
    ],
}
# Nothing of Mojang's goes in: no file the jar or the asset objects hold.
mojang = set(kinds["jar"]) | set(kinds["object"]) | set(kinds["index"])
tables = sorted(kinds["horizon"])
assert not mojang & set(tables)
assert not any(t.startswith(("assets/", "data/")) for t in tables)

fixed = (2026, 1, 1, 0, 0, 0)
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    info = zipfile.ZipInfo("ios-setup.json", fixed)
    info.compress_type = zipfile.ZIP_DEFLATED
    z.writestr(info, json.dumps(setup, separators=(",", ":")))
    for rel in tables:
        info = zipfile.ZipInfo("tables/" + rel, fixed)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o644 << 16
        z.writestr(info, open(os.path.join(prepared, rel), "rb").read())

digest = hashlib.sha256(open(out, "rb").read()).hexdigest()
total = sum(o["size"] for o in setup["objects"]) + setup["client"]["size"]
print(f"{out}: {len(tables)} table files, {os.path.getsize(out) / 1e6:.1f} MB, sha256 {digest}")
print(f"the phone downloads from Mojang: {len(setup['jar_files'])} jar files (jar {setup['client']['size'] / 1e6:.0f} MB), "
      f"{len(setup['objects'])} objects; {total / 1e6:.0f} MB in all")
