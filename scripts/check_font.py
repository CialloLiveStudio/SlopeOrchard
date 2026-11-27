"""Verify the shipped UI font. Pass a CJK font with --source to rebuild the subset."""
import argparse
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "godot/assets/SlopeOrchardSansSC.ttf"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path)
    args = parser.parse_args()
    points = set(range(32, 127))
    for folder in ("godot/scripts", "rust"):
        for path in (ROOT / folder).rglob("*"):
            if path.is_file() and path.suffix in {".gd", ".rs", ".toml", ".uid"}:
                points.update(ord(c) for c in path.read_text(encoding="utf-8-sig") if ord(c) >= 128)
    if args.source:
        with TTFont(args.source, fontNumber=0) as font:
            if points - font.getBestCmap().keys():
                parser.error("Source font lacks required characters")
            selection = subset.Subsetter()
            selection.populate(unicodes=points)
            selection.subset(font)
            font.save(DEST)
    with TTFont(DEST) as font:
        missing = [f"U+{n:04X}" for n in sorted(points - font.getBestCmap().keys())]
    print(f"MISSING_GLYPHS: {missing}")
    return int(bool(missing))


if __name__ == "__main__":
    raise SystemExit(main())
