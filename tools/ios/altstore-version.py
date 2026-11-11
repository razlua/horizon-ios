"""Adds a release's .ipa to the AltStore / SideStore source (altstore-source.json).

Usage: python tools/ios/altstore-version.py <Horizon-unsigned.ipa> <download URL> [notes]

The version, build number and privacy usage descriptions are read from the .ipa's own
Info.plist, because AltStore refuses an install whose version or permissions differ from the
source. The size is the file's. Run it for every release, with the release asset's URL.
"""

import datetime
import json
import os
import plistlib
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SOURCE = os.path.join(ROOT, "altstore-source.json")


def info_plist(ipa):
    with zipfile.ZipFile(ipa) as z:
        for name in z.namelist():
            parts = name.split("/")
            if len(parts) == 3 and parts[0] == "Payload" and parts[2] == "Info.plist":
                return plistlib.loads(z.read(name))
    sys.exit(f"{ipa}: no Payload/*.app/Info.plist")


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    ipa, url = sys.argv[1], sys.argv[2]
    notes = sys.argv[3] if len(sys.argv) > 3 else "See the release notes on GitHub."
    info = info_plist(ipa)
    with open(SOURCE, encoding="utf-8") as f:
        source = json.load(f)
    app = source["apps"][0]
    if info["CFBundleIdentifier"] != app["bundleIdentifier"]:
        sys.exit(f"bundle id {info['CFBundleIdentifier']} is not {app['bundleIdentifier']}")
    version = {
        "version": info["CFBundleShortVersionString"],
        "buildVersion": info["CFBundleVersion"],
        "date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "localizedDescription": notes,
        "downloadURL": url,
        "size": os.path.getsize(ipa),
        "minOSVersion": info.get("MinimumOSVersion", "16.0"),
    }
    same = lambda v: (v["version"], v["buildVersion"]) == (version["version"], version["buildVersion"])
    app["versions"] = [version] + [v for v in app["versions"] if not same(v)]
    app["appPermissions"] = {
        "entitlements": [],
        "privacy": {k: v for k, v in sorted(info.items()) if k.endswith("UsageDescription")},
    }
    with open(SOURCE, "w", encoding="utf-8", newline="\n") as f:
        json.dump(source, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print(f"{SOURCE}: {version['version']} ({version['buildVersion']}), {version['size']} bytes")


if __name__ == "__main__":
    main()
