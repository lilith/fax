#!/usr/bin/env bash
# Regenerate test-files/reference-hashes.tsv from libtiff (errors/*.tif)
# and committed PBMs (files/*.pbm).
#
# Requires: libtiff-tools (tifftopnm), sha256sum
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v tifftopnm &>/dev/null; then
    echo "error: tifftopnm not found (install libtiff-tools)" >&2
    exit 1
fi

OUT="test-files/reference-hashes.tsv"

cat > "$OUT" << 'HEADER'
# SHA-256 hashes of correctly decoded PBM output (P4 format).
# Generated from libtiff (tifftopnm) for errors/*.tif, and from
# committed PBMs for files/*.pbm. Regenerate with scripts/regen-hashes.sh.
#
# id	dir	width	height	sha256
HEADER

# errors/*.tif -> decode with libtiff, hash the PBM output
for tif in test-files/errors/*.tif; do
    id=$(basename "$tif" .tif)
    pbm=$(tifftopnm "$tif" 2>/dev/null)
    dims=$(echo "$pbm" | head -2 | tail -1)
    width=$(echo "$dims" | cut -d' ' -f1)
    height=$(echo "$dims" | cut -d' ' -f2)
    hash=$(echo "$pbm" | sha256sum | cut -d' ' -f1)
    printf '%s\terrors\t%s\t%s\t%s\n' "$id" "$width" "$height" "$hash"
done >> "$OUT"

# files/*.pbm -> hash the committed reference directly
for pbm_file in test-files/files/*.pbm; do
    stem=$(basename "$pbm_file" .pbm)
    # Only include files that have a corresponding .fax or .tiff
    if [ ! -f "test-files/files/${stem}.fax" ] && [ ! -f "test-files/files/${stem}.tiff" ]; then
        continue
    fi
    dims=$(head -2 "$pbm_file" | tail -1)
    width=$(echo "$dims" | cut -d' ' -f1)
    height=$(echo "$dims" | cut -d' ' -f2)
    hash=$(sha256sum "$pbm_file" | cut -d' ' -f1)
    printf '%s\tfiles\t%s\t%s\t%s\n' "$stem" "$width" "$height" "$hash"
done >> "$OUT"

echo "wrote $(grep -cv '^#' "$OUT") entries to $OUT"
