//! Deterministic PRNG for v3 simulators.
//!
//! xorshift64 — fast, seedable, no external crates (mirrors v2's
//! zero-dependency policy). Not cryptographic. Identical seeds reproduce
//! identical sequences across machines, which the rename-invariance
//! benchmark relies on.

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 0xDEAD_BEEF_DEAD_BEEF } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Uniform sample in `[0, 1)`.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Box-Muller normal sample.
    pub fn normal(&mut self, mu: f64, sigma: f64) -> f64 {
        let u1 = self.uniform().max(1e-12);
        let u2 = self.uniform();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        mu + sigma * z
    }
}
