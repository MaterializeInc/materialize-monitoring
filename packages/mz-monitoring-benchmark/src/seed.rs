// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Stable, seedable hashing.
//!
//! Everything a pod generates is a pure function of its seed: shard IDs, the
//! jitter on each series, the noise on each scrape. A pod that restarts with
//! the same seed therefore serves the same identities, and no per-series state
//! has to be kept. `std`'s hashers are not stable across Rust releases, so this
//! is FNV-1a for strings and SplitMix64 for mixing.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a over `bytes`.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// One SplitMix64 step: a cheap bijective mixer with good avalanche.
pub fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(u64);

impl Seed {
    pub fn new(seed: &str) -> Self {
        Seed(splitmix(fnv1a(seed.as_bytes())))
    }

    /// An independent stream for one purpose, so that adding a use of the seed
    /// never shifts the values another use already produced.
    pub fn derive(self, tag: &str) -> Self {
        Seed(splitmix(self.0 ^ fnv1a(tag.as_bytes())))
    }

    pub fn hash(self, i: u64) -> u64 {
        splitmix(self.0 ^ splitmix(i))
    }

    /// Hash a tuple. Order matters: `(1, 2)` and `(2, 1)` differ.
    pub fn mix(self, parts: &[u64]) -> u64 {
        parts.iter().fold(self.0, |h, p| splitmix(h ^ splitmix(*p)))
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(self, i: u64) -> f64 {
        to_unit(self.hash(i))
    }

    /// Uniform in `[1 - amount, 1 + amount]`.
    pub fn jitter(self, i: u64, amount: f64) -> f64 {
        1.0 + amount * (2.0 * self.unit(i) - 1.0)
    }

    /// A version-4-shaped UUID, lowercase, hyphenated.
    pub fn uuid(self, i: u64) -> String {
        let hi = self.hash(i);
        let lo = self.hash(i ^ 0x5555_5555_5555_5555).rotate_left(29) ^ hi.rotate_right(7);
        let bytes = (u128::from(hi) << 64 | u128::from(lo)).to_be_bytes();
        uuid::Builder::from_random_bytes(bytes)
            .into_uuid()
            .hyphenated()
            .to_string()
    }
}

/// The top 53 bits of `h` as a float in `[0, 1)`.
pub fn to_unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Scale `n` by a seeded factor in `[1 - amount, 1 + amount]`, never below 1
/// when `n` is at least 1.
pub fn jitter_count(seed: Seed, tag: &str, n: u64, amount: f64) -> u64 {
    if n == 0 {
        return 0;
    }
    let scaled = (n as f64 * seed.derive(tag).jitter(0, amount)).round() as u64;
    scaled.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_identities() {
        let a = Seed::new("bench-environmentd-0").derive("shard");
        let b = Seed::new("bench-environmentd-0").derive("shard");
        assert_eq!(a.uuid(7), b.uuid(7));
        assert_ne!(a.uuid(7), a.uuid(8));
        assert_ne!(
            a.uuid(7),
            Seed::new("bench-environmentd-1").derive("shard").uuid(7)
        );
    }

    #[test]
    fn uuid_is_version_4_shaped() {
        let u = Seed::new("x").uuid(0);
        let re = regex::Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
        )
        .unwrap();
        assert!(re.is_match(&u), "{u}");
    }

    /// Pinned, because a change here re-keys every pod's identities: a fleet
    /// rolled onto a new image would churn every series at once. The values
    /// were computed by a separate implementation of FNV-1a and SplitMix64.
    #[test]
    fn identities_are_stable_across_releases() {
        assert_eq!(fnv1a(b"materialize"), 0x2fa8_a7f3_ce15_d112);
        assert_eq!(
            Seed::new("bench-environmentd-0").derive("shard").uuid(0),
            "dde8c296-6d0b-4ea7-ba55-ceb7eb36d236"
        );
    }

    #[test]
    fn jitter_stays_in_bounds() {
        let s = Seed::new("j");
        for i in 0..10_000 {
            let j = s.jitter(i, 0.1);
            assert!((0.9..=1.1).contains(&j), "{j}");
        }
    }

    #[test]
    fn jittered_counts_stay_near_the_request() {
        for name in ["a", "b", "c", "d"] {
            let n = jitter_count(Seed::new(name), "collections", 25_000, 0.1);
            assert!((22_500..=27_500).contains(&n), "{n}");
        }
        assert_eq!(jitter_count(Seed::new("a"), "x", 0, 0.1), 0);
        assert_eq!(jitter_count(Seed::new("a"), "x", 1, 0.1), 1);
    }
}
