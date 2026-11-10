#!/usr/bin/env python3
"""Adds your own prepared Minecraft resources to an unsigned Horizon .ipa.

Horizon ships none of Mojang's files, so the .ipa a build produces cannot start a world on
its own. This puts the resources the launcher prepared on your computer (from your copy of
Minecraft) into the app as one `resources.zip`, which the game unpacks at its first start.
The result is for your own device: do not share it.

    python tools/ios/add-resources.py Horizon-unsigned.ipa RESOURCES_DIR Horizon.ipa

RESOURCES_DIR is the folder holding `horizon-resources.json`, for example
<launcher data>/versions/<version>/.local/horizon-client/26.3
"""
import io
import os
import sys
import zipfile

APP = "Payload/Horizon.app/"
# Left on the computer: the Java exporters' classes and logs (the game reads neither), and
# `horizon-data.bin`, which is tied to the folder it was written in (the game writes its own).
SKIP_DIRS = {"classes", "client-classes"}
SKIP_SUFFIXES = (".log", ".tmp")
SKIP_FILES = {"horizon-data.bin", ".unpacked-from", ".horizon-prepare.lock"}
# Already compressed: stored as they are, which unpacks faster on the phone.
STORED = (".png", ".ogg", ".jpg", ".jpeg", ".webp", ".zip", ".jar", ".gz")


def resource_files(root):
    for folder, dirs, files in os.walk(root):
        rel = os.path.relpath(folder, root)
        if rel == ".":
            dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        dirs.sort()
        for name in sorted(files):
            if name in SKIP_FILES or name.endswith(SKIP_SUFFIXES) or ".tmp" in name:
                continue
            path = os.path.join(folder, name)
            yield path, os.path.relpath(path, root).replace(os.sep, "/")


def main(argv):
    if len(argv) != 4:
        sys.exit(__doc__)
    source, root, out = argv[1], argv[2], argv[3]
    if not os.path.isfile(os.path.join(root, "horizon-resources.json")):
        sys.exit(f"{root} holds no horizon-resources.json: not a prepared resources folder")
    inner = io.BytesIO()
    count = size = 0
    with zipfile.ZipFile(inner, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for path, name in resource_files(root):
            how = zipfile.ZIP_STORED if name.lower().endswith(STORED) else zipfile.ZIP_DEFLATED
            z.write(path, name, compress_type=how)
            count += 1
            size += os.path.getsize(path)
    packed = inner.getvalue()
    with zipfile.ZipFile(source) as src, zipfile.ZipFile(out, "w") as dst:
        names = src.namelist()
        if APP + "horizon-client" not in names:
            sys.exit(f"{source} is not a Horizon .ipa (no {APP}horizon-client)")
        for info in src.infolist():
            # The signature no longer matches once a file is added: sideloading signs again.
            if info.filename.startswith(APP + "_CodeSignature/"):
                continue
            if info.filename == APP + "resources.zip":
                continue
            # The ZipInfo carries the Unix mode: the program stays executable.
            dst.writestr(info, src.read(info.filename), compress_type=zipfile.ZIP_DEFLATED)
        entry = zipfile.ZipInfo(APP + "resources.zip")
        entry.external_attr = 0o644 << 16
        entry.create_system = 3
        dst.writestr(entry, packed, compress_type=zipfile.ZIP_STORED)
    print(
        f"{out}: {count} resource files ({size / 1e6:.0f} MB) packed into "
        f"{len(packed) / 1e6:.0f} MB; the .ipa is {os.path.getsize(out) / 1e6:.0f} MB"
    )


if __name__ == "__main__":
    main(sys.argv)
