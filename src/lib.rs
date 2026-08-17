//! Every จังหวัด, อำเภอ and ตำบล in Thailand, and what to do with an address
//! that nearly matches one.
//!
//! # What this is for
//!
//! Thai addresses arrive misspelled. A photograph of an ID card loses the tone
//! mark off the top of a letter; a card prints `อ.เมือง` where the published
//! list holds `เมืองเชียงใหม่`; somebody types `ศรี สะเกษ` with a space. All
//! three name a real place, and a form that cannot tell them from nonsense
//! makes a person check every box by hand.
//!
//! So this answers one question — *does this combination exist, and if not,
//! is it obviously one that does?* — and it is careful about when to say yes.
//!
//! ```
//! use thai_geography::{Address, Geography, Verdict};
//!
//! let geo = Geography::bundled();
//! let checked = geo.check(&Address {
//!     sub_district: "พระสิงห".into(),   // a tone mark short
//!     district: "เมือง".into(),        // the short form a card prints
//!     province: "เชียงใหม่".into(),
//! });
//!
//! assert_eq!(checked.verdict, Verdict::Corrected);
//! assert_eq!(checked.address.sub_district, "พระสิงห์");
//! assert_eq!(checked.address.district, "เมืองเชียงใหม่");
//! ```
//!
//! # What it will not do
//!
//! It will not guess. A ตำบล two edits away from three different real ones is
//! not a near miss, and a district that belongs to another province is not
//! relocated into the one it was written beside. Both come back
//! [`Verdict::Unknown`] with the address untouched, because a wrong address
//! asserted confidently is worse than one that admits it cannot be placed.
//!
//! # Where the data comes from
//!
//! [thailand-geography-data/thailand-geography-json][src] (MIT): 77 provinces,
//! 928 districts, 7,436 subdistricts. Thai names only — this crate answers one
//! question, and the codes, postal codes and Latin names in the upstream set
//! are weight that does not help answer it.
//!
//! `tool/build.sh` regenerates `data/thai_geography.json` and refuses to write
//! a file whose counts have collapsed, which is what an upstream schema change
//! looks like from here.
//!
//! [src]: https://github.com/thailand-geography-data/thailand-geography-json

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The three parts a published list can check.
///
/// หมู่ที่ and บ้านเลขที่ are not among them: no list holds house numbers, and
/// a village number is a number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub sub_district: String,
    pub district: String,
    pub province: String,
}

/// Which part a check replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Province,
    District,
    SubDistrict,
}

/// What the list made of an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every part named, and the combination exists, spelled as the list
    /// spells it. This is the answer that saves somebody work.
    Found,
    /// The combination exists, but not as it was written. At least one part
    /// has been replaced — still worth showing to whoever wrote it, since a
    /// list is not the last word on where somebody lives.
    Corrected,
    /// No combination like this exists, and nothing close enough to name.
    Unknown,
    /// Not enough was written to look anything up. A blank address is not a
    /// wrong address.
    Incomplete,
}

/// An address, checked.
#[derive(Debug, Clone)]
pub struct Checked {
    pub verdict: Verdict,
    /// As the list spells it when it could be placed; otherwise exactly what
    /// came in.
    pub address: Address,
    /// Which parts were replaced, sorted.
    pub changed: Vec<Part>,
}

impl Checked {
    /// Whether the address can be used — found, or corrected into something
    /// that exists.
    pub fn is_usable(&self) -> bool {
        matches!(self.verdict, Verdict::Found | Verdict::Corrected)
    }
}

/// The list.
pub struct Geography {
    /// province → district → subdistricts.
    tree: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

/// The dataset, compiled in. ~215KB, so a consumer needs no data files and no
/// network — which is the point for a phone filling in a form on mobile data.
const BUNDLED: &str = include_str!("../data/thai_geography.json");

static INSTANCE: OnceLock<Geography> = OnceLock::new();

impl Geography {
    /// The bundled list, parsed once per process.
    ///
    /// # Panics
    ///
    /// If the compiled-in dataset does not parse. That is a build-time fact,
    /// not a runtime condition: a consumer cannot supply a broken one, and
    /// making every caller handle an error that can only fire if this crate
    /// shipped broken would be noise in a thousand call sites.
    pub fn bundled() -> &'static Geography {
        INSTANCE
            .get_or_init(|| Geography::from_json(BUNDLED).expect("the bundled dataset must parse"))
    }

    /// Parse a list in the same shape as `data/thai_geography.json` —
    /// `{province: {district: [subdistrict, ...]}}`.
    ///
    /// For a consumer that keeps its own copy, and for the tests that check
    /// behaviour without depending on a real place existing.
    pub fn from_json(raw: &str) -> Result<Geography, serde_json::Error> {
        Ok(Geography {
            tree: serde_json::from_str(raw)?,
        })
    }

    /// Every จังหวัด.
    pub fn provinces(&self) -> impl Iterator<Item = &str> {
        self.tree.keys().map(String::as_str)
    }

    /// The อำเภอ of a province, or nothing if that is not a province.
    pub fn districts_in(&self, province: &str) -> impl Iterator<Item = &str> {
        self.tree
            .get(province)
            .into_iter()
            .flat_map(|d| d.keys().map(String::as_str))
    }

    /// The ตำบล of a district.
    pub fn sub_districts_in(&self, province: &str, district: &str) -> impl Iterator<Item = &str> {
        self.tree
            .get(province)
            .and_then(|d| d.get(district))
            .into_iter()
            .flat_map(|s| s.iter().map(String::as_str))
    }

    /// Whether this exact combination exists, with no correction at all.
    pub fn holds(&self, address: &Address) -> bool {
        self.tree
            .get(&address.province)
            .and_then(|d| d.get(&address.district))
            .is_some_and(|subs| subs.iter().any(|s| s == &address.sub_district))
    }

    /// Check an address, correcting a near miss.
    ///
    /// Top down, because the parts are not independent: the same ตำบล name
    /// occurs in dozens of อำเภอ across the country, so resolving the province
    /// first turns "one of 7,436" into "one of the twelve in this district".
    /// That is faster, and far more accurate — a one-character miss that is
    /// ambiguous nationally is usually unique locally.
    pub fn check(&self, address: &Address) -> Checked {
        let unchanged = |verdict| Checked {
            verdict,
            address: address.clone(),
            changed: Vec::new(),
        };

        if address.province.trim().is_empty()
            || address.district.trim().is_empty()
            || address.sub_district.trim().is_empty()
        {
            return unchanged(Verdict::Incomplete);
        }

        let mut changed = Vec::new();

        let Some(province) = closest(&address.province, self.tree.keys().map(String::as_str))
        else {
            return unchanged(Verdict::Unknown);
        };
        if province != address.province {
            changed.push(Part::Province);
        }

        let districts = &self.tree[province];
        let Some(district) = closest(&address.district, districts.keys().map(String::as_str))
        else {
            return unchanged(Verdict::Unknown);
        };
        if district != address.district {
            changed.push(Part::District);
        }

        let Some(sub_district) = closest(
            &address.sub_district,
            districts[district].iter().map(String::as_str),
        ) else {
            return unchanged(Verdict::Unknown);
        };
        if sub_district != address.sub_district {
            changed.push(Part::SubDistrict);
        }

        changed.sort_unstable();
        Checked {
            verdict: if changed.is_empty() {
                Verdict::Found
            } else {
                Verdict::Corrected
            },
            address: Address {
                sub_district: sub_district.to_string(),
                district: district.to_string(),
                province: province.to_string(),
            },
            changed,
        }
    }
}

/// The one entry `written` plainly means, or nothing.
///
/// Exact first. Then the administrative prefixes a card prints and a list does
/// not — `เมือง` for `เมืองเชียงใหม่` — because that is not a misreading and
/// correcting it as one would be luck.
///
/// Then, and only then, one edit. ONE, and only when it is unambiguous: the
/// whole value of an automatic correction is that nobody has to check it, so a
/// correction that might be wrong is worth less than no correction at all.
fn closest<'a>(written: &str, options: impl Iterator<Item = &'a str> + Clone) -> Option<&'a str> {
    let target = fold(written);
    if target.is_empty() {
        return None;
    }

    let mut prefixed: Option<&str> = None;
    for option in options.clone() {
        let folded = fold(option);
        if folded == target {
            return Some(option);
        }
        if folded.starts_with(&target) || target.starts_with(&folded) {
            // An ambiguous prefix is no better than an ambiguous edit.
            if prefixed.is_some() {
                return None;
            }
            prefixed = Some(option);
        }
    }
    if prefixed.is_some() {
        return prefixed;
    }

    let mut near: Option<&str> = None;
    for option in options {
        if !within_one_edit(&fold(option), &target) {
            continue;
        }
        if near.is_some() {
            return None;
        }
        near = Some(option);
    }
    near
}

/// A comparison key that ignores what an OCR pass and a typist get wrong about
/// Thai without changing which place is meant.
///
/// Tone marks and the silent การันต์ come off: they sit above and below the
/// line, they are the first thing a compression artefact eats, and no two Thai
/// place names differ only by one. Spaces go too — `ศรีสะเกษ` and `ศรี สะเกษ`
/// are the same province typed by two people.
///
/// Deliberately does NOT fold consonants that merely look alike (บ/ป, ด/ค,
/// ถ/ภ). They are different letters, real pairs of places differ by exactly
/// one of them, and folding would make those pairs ambiguous — so the
/// correction would refuse in exactly the cases it is most wanted.
fn fold(value: &str) -> Vec<char> {
    value
        .trim()
        .chars()
        .filter(|c| !matches!(*c, '\u{0E47}'..='\u{0E4E}' | ' '))
        .collect()
}

/// True when the two are at most one insertion, deletion or substitution
/// apart.
///
/// Not a full Levenshtein: the answer is only ever used as `<= 1`, and a
/// bounded check is one pass instead of a matrix — run against up to 7,436
/// candidates in the worst case.
fn within_one_edit(a: &[char], b: &[char]) -> bool {
    if a == b {
        return true;
    }
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let (shorter, longer) = if a.len() <= b.len() { (a, b) } else { (b, a) };

    let (mut i, mut j) = (0usize, 0usize);
    let mut slack = true;
    while i < shorter.len() && j < longer.len() {
        if shorter[i] == longer[j] {
            i += 1;
            j += 1;
            continue;
        }
        if !slack {
            return false;
        }
        slack = false;
        // Equal lengths mean the differing character was substituted;
        // otherwise the longer string has one the shorter does not.
        if shorter.len() == longer.len() {
            i += 1;
        }
        j += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(sub_district: &str, district: &str, province: &str) -> Address {
        Address {
            sub_district: sub_district.into(),
            district: district.into(),
            province: province.into(),
        }
    }

    #[test]
    fn the_bundled_list_is_whole() {
        // Against the real data, because a list that answered correctly about
        // six invented provinces would prove nothing about the one that ships
        // — and an upstream schema change that empties half the rows is
        // invisible to a fixture.
        let geo = Geography::bundled();
        assert_eq!(geo.provinces().count(), 77);

        let districts: usize = geo.provinces().map(|p| geo.districts_in(p).count()).sum();
        assert_eq!(districts, 928);

        let subs: usize = geo
            .provinces()
            .flat_map(|p| {
                geo.districts_in(p)
                    .map(move |d| geo.sub_districts_in(p, d).count())
            })
            .sum();
        assert_eq!(subs, 7436);
    }

    #[test]
    fn an_address_that_is_right_is_left_alone() {
        let checked = Geography::bundled().check(&at("พระสิงห์", "เมืองเชียงใหม่", "เชียงใหม่"));
        assert_eq!(checked.verdict, Verdict::Found);
        assert!(checked.changed.is_empty());
    }

    #[test]
    fn bangkok_is_the_same_three_levels() {
        let checked = Geography::bundled().check(&at("ลุมพินี", "ปทุมวัน", "กรุงเทพมหานคร"));
        assert_eq!(checked.verdict, Verdict::Found);
    }

    #[test]
    fn the_short_form_a_card_prints_is_not_an_error() {
        // The card prints อ.เมือง and the list holds the long name. A form
        // that reported an error here would report one on a correctly read
        // address from half the country.
        let checked = Geography::bundled().check(&at("พระสิงห์", "เมือง", "เชียงใหม่"));
        assert_eq!(checked.verdict, Verdict::Corrected);
        assert_eq!(checked.address.district, "เมืองเชียงใหม่");
        assert_eq!(checked.changed, vec![Part::District]);
    }

    #[test]
    fn a_tone_mark_the_photograph_lost_is_put_back() {
        let checked = Geography::bundled().check(&at("พระสิงห", "เมืองเชียงใหม่", "เชียงใหม่"));
        assert_eq!(checked.verdict, Verdict::Corrected);
        assert_eq!(checked.address.sub_district, "พระสิงห์");
    }

    #[test]
    fn a_space_somebody_typed_is_not_a_different_province() {
        let checked = Geography::bundled().check(&at("เมืองใต้", "เมืองศรีสะเกษ", "ศรี สะเกษ"));
        assert_eq!(checked.verdict, Verdict::Corrected);
        assert_eq!(checked.address.province, "ศรีสะเกษ");
    }

    #[test]
    fn a_district_from_another_province_is_not_relocated() {
        // ปทุมวัน is Bangkok's. Asked for inside เชียงใหม่ it does not exist,
        // and the honest answer is that this cannot be placed.
        let checked = Geography::bundled().check(&at("ลุมพินี", "ปทุมวัน", "เชียงใหม่"));
        assert_eq!(checked.verdict, Verdict::Unknown);
        assert_eq!(checked.address.district, "ปทุมวัน", "left as written");
    }

    #[test]
    fn nonsense_is_unknown_not_the_nearest_thing_to_nonsense() {
        let checked = Geography::bundled().check(&at("xxxxxxxx", "yyyyyyyy", "zzzzzzzz"));
        assert_eq!(checked.verdict, Verdict::Unknown);
    }

    #[test]
    fn a_half_typed_address_is_incomplete_never_wrong() {
        let geo = Geography::bundled();
        assert_eq!(
            geo.check(&at("", "", "เชียงใหม่")).verdict,
            Verdict::Incomplete
        );
        assert_eq!(geo.check(&at("", "", "")).verdict, Verdict::Incomplete);
    }

    #[test]
    fn two_candidates_one_edit_away_is_not_a_near_miss() {
        // The property that makes an automatic correction safe to apply
        // without anybody checking it. Against a made-up list, because the
        // real one must not be relied on to contain an ambiguous pair.
        let geo = Geography::from_json(r#"{"ก":{"ข":["กาก","กาข","ขบ"]}}"#).unwrap();
        // "กาค" is one edit from both "กาก" and "กาข", so it names neither.
        assert_eq!(geo.check(&at("กาค", "ข", "ก")).verdict, Verdict::Unknown);
        // "กบ" is one edit from "ขบ" and no other, so it names that one.
        let checked = geo.check(&at("กบ", "ข", "ก"));
        assert_eq!(checked.verdict, Verdict::Corrected);
        assert_eq!(checked.address.sub_district, "ขบ");
    }

    #[test]
    fn holds_asks_the_exact_question() {
        let geo = Geography::bundled();
        assert!(geo.holds(&at("พระสิงห์", "เมืองเชียงใหม่", "เชียงใหม่")));
        assert!(!geo.holds(&at("พระสิงห", "เมืองเชียงใหม่", "เชียงใหม่")));
    }
}
