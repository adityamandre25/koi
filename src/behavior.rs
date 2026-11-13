//! Swimming behaviour chosen from yesterday's contributions.
//!
//! - 0: peaceful gliding
//! - 1–3: gentle swimming
//! - 4–10: energetic swimming
//! - 11+: fast swimming with stronger tail beats
//!
//! Includes Sunday. Daily activity never affects permanent body growth.

use crate::koi::KoiDevelopment;

/// Swimming behaviour for the day, chosen from yesterday's contributions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Behavior {
    /// 0 contributions yesterday: peaceful gliding.
    PeacefulGliding,
    /// 1–3 contributions yesterday: gentle swimming.
    GentleSwimming,
    /// 4–10 contributions yesterday: energetic swimming.
    EnergeticSwimming,
    /// 11+ contributions yesterday: fast swimming with stronger tail beats.
    FastSwimming,
}

/// Choose the movement for the day based on yesterday's contributions:
///
/// | Yesterday's contributions | Behaviour |
/// | --- | --- |
/// | 0 | Peaceful gliding |
/// | 1–3 | Gentle swimming |
/// | 4–10 | Energetic swimming |
/// | 11+ | Fast swimming with stronger tail beats |
///
/// Includes Sunday. Daily activity never affects permanent body growth.
pub fn select_behavior(koi_development: &KoiDevelopment) -> Behavior {
    match koi_development.yesterday_contributions {
        0 => Behavior::PeacefulGliding,
        1..=3 => Behavior::GentleSwimming,
        4..=10 => Behavior::EnergeticSwimming,
        _ => Behavior::FastSwimming,
    }
}
