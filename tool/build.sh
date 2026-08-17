#!/usr/bin/env bash
# Rebuild `data/thai_geography.json` from the upstream dataset.
#
# Source: thailand-geography-data/thailand-geography-json (MIT).
#
# Thai names only, and no codes or postal codes. This file exists to answer
# one question — does this ตำบล/อำเภอ/จังหวัด combination exist — and every
# column that does not help answer it is weight in the app bundle. Add them
# back the day something needs them.
#
# Run when the Ministry of Interior publishes a change, which is a handful of
# subdistricts a year, and commit the result: the dataset is compiled into the
# crate, so a build must never need the network.
set -euo pipefail

HERE=$(cd "$(dirname "$0")/.." && pwd)
DEST=$HERE/data/thai_geography.json
SRC=${1:-https://raw.githubusercontent.com/thailand-geography-data/thailand-geography-json/main/src/geography.json}
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

curl -sSL --max-time 120 -o "$TMP" "$SRC"

python3 - "$TMP" "$DEST" <<'PY'
import collections, json, sys

rows = json.load(open(sys.argv[1]))
tree = collections.OrderedDict()
for row in rows:
    province = tree.setdefault(row['provinceNameTh'], collections.OrderedDict())
    district = province.setdefault(row['districtNameTh'], [])
    if row['subdistrictNameTh'] not in district:
        district.append(row['subdistrictNameTh'])

provinces = len(tree)
districts = sum(len(d) for d in tree.values())
subdistricts = sum(len(s) for d in tree.values() for s in d.values())
# A dataset that lost half its rows to an upstream schema change must not be
# committed quietly. These are the published counts, with room to move.
assert 76 <= provinces <= 78, f'{provinces} provinces'
assert 900 <= districts <= 960, f'{districts} districts'
assert 7300 <= subdistricts <= 7600, f'{subdistricts} subdistricts'

with open(sys.argv[2], 'w') as out:
    json.dump(tree, out, ensure_ascii=False, separators=(',', ':'))
print(f'{provinces} provinces, {districts} districts, {subdistricts} subdistricts')
PY
echo "wrote $DEST"
