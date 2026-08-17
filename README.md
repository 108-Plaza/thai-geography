# thai-geography

Every จังหวัด, อำเภอ and ตำบล in Thailand — 77 provinces, 928 districts, 7,436
subdistricts — and correction of an address that nearly matches one.

```rust
use thai_geography::{Address, Geography, Verdict};

let geo = Geography::bundled();
let checked = geo.check(&Address {
    sub_district: "พระสิงห".into(),   // a tone mark short
    district: "เมือง".into(),        // the short form a card prints
    province: "เชียงใหม่".into(),
});

assert_eq!(checked.verdict, Verdict::Corrected);
assert_eq!(checked.address.sub_district, "พระสิงห์");
assert_eq!(checked.address.district, "เมืองเชียงใหม่");
```

## Why

Thai addresses arrive misspelled. A photograph of an ID card loses the tone
mark off the top of a letter. A card prints `อ.เมือง` where the published list
holds `เมืองเชียงใหม่`. Somebody types `ศรี สะเกษ` with a space. All three
name a real place, and a form that cannot tell them apart from nonsense makes a
person check every box by hand.

This was written for a rider application form: an ID card is photographed, its
address is read by OCR, and every value arrives needing a human to agree to it
— four boxes, on a phone, in a script where the difference between **พระสิงห์**
and **พระสิงห** is a mark above the last letter that nobody sees at arm's
length. Most of that checking is work a list can do.

## Three answers

| verdict | meaning |
|---|---|
| `Found` | the combination exists as written |
| `Corrected` | a character or tone mark out, or the short form a card prints — replaced with the list's spelling |
| `Unknown` | no combination like this, and nothing close enough to name |
| `Incomplete` | not enough written to look anything up |

### It will not guess

A ตำบล two edits from three different real ones is not a near miss. A district
belonging to another province is not relocated into the one it was written
beside. Both come back `Unknown` **with the address untouched**, because a
wrong address asserted confidently is worse than one that admits it cannot be
placed.

`Corrected` is still worth showing to whoever wrote it. A published list is not
the last word on where somebody lives.

## How the matching works

**Top down**, because the parts are not independent: the same ตำบล name occurs
in dozens of อำเภอ across the country. Resolving the province first turns *"one
of 7,436"* into *"one of the twelve in this district"* — faster, and far more
accurate, since a one-character miss that is ambiguous nationally is usually
unique locally.

The comparison **drops tone marks, การันต์ and spaces**. They sit above and
below the line, they are the first thing a compression artefact eats, and no
two Thai place names differ only by one.

It deliberately does **not** fold consonants that merely look alike (บ/ป, ด/ค,
ถ/ภ). Those are different letters, real pairs of places differ by exactly one
of them, and folding would make those pairs ambiguous — so the correction would
refuse in exactly the cases it is most wanted.

Only **one** edit, and only when unambiguous. The whole value of an automatic
correction is that nobody has to check it, so a correction that might be wrong
is worth less than no correction at all.

## The data

Generated from
[thailand-geography-data/thailand-geography-json](https://github.com/thailand-geography-data/thailand-geography-json)
(MIT). Thai names only — this crate answers one question, and the codes, postal
codes and Latin names in the upstream set are weight that does not help answer
it. ~215KB, compiled into the binary with `include_str!`, so a consumer needs
no data files and no network.

```sh
./tool/build.sh          # regenerate data/thai_geography.json
```

It refuses to write a file whose counts have collapsed, which is what an
upstream schema change looks like from here. Run it when the Ministry of
Interior publishes a change — a handful of subdistricts a year — and commit the
result.

## Licence

MIT. The dataset is MIT and its notice is reproduced in [LICENSE](LICENSE).
