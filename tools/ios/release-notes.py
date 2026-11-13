"""Prints a release's notes: its sections of CHANGELOG.md, then how to install and update.

Usage: python tools/ios/release-notes.py <section heading> [more headings...]
e.g.   python tools/ios/release-notes.py 2026.11.12 > notes.md
       python tools/ios/release-notes.py Unreleased > notes.md   (what changed since the latest release)

When a release goes out, its notes are the Unreleased section; then that heading becomes the
release's version (`## 2026.11.13`) and a new, empty Unreleased goes above it.
"""

import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

FOOTER = """
---

**Install:** download `Horizon-unsigned.ipa` below and follow
[Installing](https://github.com/razlua/horizon-ios#installing), or add the AltStore / SideStore
source: `https://raw.githubusercontent.com/razlua/horizon-ios/ios-port/altstore-source.json`

**Updating:** install over the old app with the same Apple ID. Don't delete it first, or your
worlds and game files go with it.

Ferrite is an unofficial port of [Horizon](https://github.com/horizon-rust/horizon), not affiliated with
Horizon, Mojang, Microsoft or Simple Voice Chat. Bugs: @4rnz on Discord.
"""


def section(text, heading):
    lines = text.splitlines()
    start = next((i for i, l in enumerate(lines) if l.strip() == f"## {heading}"), None)
    if start is None:
        sys.exit(f"CHANGELOG.md has no section '## {heading}'")
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
    return "\n".join(lines[start + 1 : end]).strip()


def unwrap(text):
    """Joins the lines wrapped at 100 columns: GitHub shows each line break of a release's notes."""
    out = []
    for line in text.splitlines():
        joins = (
            out
            and out[-1].strip()
            and line.strip()
            and not line.lstrip().startswith(("- ", "#", "|", "---"))
            and not out[-1].startswith(("#", "---"))
        )
        if joins:
            out[-1] = out[-1].rstrip() + " " + line.strip()
        else:
            out.append(line)
    return "\n".join(out)


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    with open(os.path.join(ROOT, "CHANGELOG.md"), encoding="utf-8") as f:
        text = f.read()
    parts = [section(text, h) for h in sys.argv[1:]]
    # The sections' own "### Added" headings, one level down in a release.
    body = "\n\n".join(parts).replace("### ", "## ")
    sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    sys.stdout.write(unwrap(body + "\n" + FOOTER) + "\n")


if __name__ == "__main__":
    main()
