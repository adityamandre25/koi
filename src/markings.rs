//! How strongly weekly markings, streaks, and record marks show on the koi.

use crate::koi::{KoiDevelopment, RecentStreak};

const STREAK_FIN_FULL_WEEKS: f64 = 13.0;
const FADE_RATE_PER_WEEK: f64 = 0.5;
const FADE_WEEKS: f64 = 6.5;
/// Shapes the longest-streak log curve: smaller values rise faster at first.
const RECORD_CURVE_WEEKS: f64 = 0.6;
const NORMAL_COLOR_WEEKS: f64 = 4.0;
const VIVID_COLOR_WEEKS: f64 = 52.0;
/// A placeholder for when glimmer reaches full strength.
const FULL_GLIMMER_WEEKS: f64 = 208.0;

/// Streak mark strengths for the renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreakMarks {
    /// Share of fin pixels in the body's streak color, from zero through one.
    pub streak_fin_strength: f64,
    /// Language color vibrancy: 0 dull, 1 normal, 2 extra bright and vivid.
    pub color_vibrancy: f64,
    /// Gin-rin glimmer on the color patches, from zero through one.
    pub glimmer_strength: f64,
    /// Weekly marking visibility, from 0.20 (very subtle) to 1.0 (maximum visibility).
    pub marking_visibility: f64,
    /// Consecutive active day marking sharpness, from 0.10 (soft) to 1.0 (maximum detail).
    pub marking_sharpness: f64,
}

/// Calculate marking visibility from weekly active days Monday–Saturday (0..=6):
/// - 0 days: very subtle markings (0.20)
/// - 1–2 days: light markings (0.38 - 0.48)
/// - 3–4 days: clear markings (0.68 - 0.78)
/// - 5 days: strong markings (0.90)
/// - 6 days: maximum visibility (1.00)
pub fn calculate_marking_visibility(weekly_active_days: u8) -> f64 {
    match weekly_active_days {
        0 => 0.20,
        1 => 0.38,
        2 => 0.48,
        3 => 0.68,
        4 => 0.78,
        5 => 0.90,
        _ => 1.00,
    }
}

/// Calculate marking sharpness from consecutive active days (0..=6):
/// - 0: soft markings (0.10)
/// - 2: clearer markings (0.50)
/// - 4: sharp markings (0.80)
/// - 6: maximum detail (1.00)
///
/// Independent of weekly marking visibility.
pub fn calculate_marking_sharpness(consecutive_active_days: u8) -> f64 {
    match consecutive_active_days {
        0 => 0.10,
        1 => 0.30,
        2 => 0.50,
        3 => 0.65,
        4 => 0.80,
        5 => 0.92,
        _ => 1.00,
    }
}

/// Derive the streak marks from a development, measuring time against its
/// observation date.
pub fn calculate_streak_marks(koi_development: &KoiDevelopment) -> StreakMarks {
    let streak_fin_strength = match &koi_development.streaks.recent {
        None => 0.0,
        Some(RecentStreak::Current(streak_weeks)) => calculate_fin_build(streak_weeks.len()),
        Some(RecentStreak::Ended {
            weeks: streak_weeks,
            ended_on,
        }) => {
            let weeks_since_end =
                (koi_development.observed_on - *ended_on).num_days().max(0) as f64 / 7.0;
            calculate_fin_build(streak_weeks.len()) * calculate_fade(weeks_since_end)
        }
    };
    let record_strength = calculate_record_strength(koi_development.streaks.longest.len() as f64);
    let normal_strength = calculate_record_strength(NORMAL_COLOR_WEEKS);
    let vivid_strength = calculate_record_strength(VIVID_COLOR_WEEKS);
    let color_vibrancy = if record_strength < normal_strength {
        record_strength / normal_strength
    } else {
        1.0 + ((record_strength - normal_strength) / (vivid_strength - normal_strength)).min(1.0)
    };
    let marking_visibility = calculate_marking_visibility(koi_development.weekly_active_days);
    let marking_sharpness = calculate_marking_sharpness(koi_development.consecutive_active_days);
    StreakMarks {
        streak_fin_strength,
        color_vibrancy,
        glimmer_strength: ((record_strength - vivid_strength) / (1.0 - vivid_strength))
            .clamp(0.0, 1.0),
        marking_visibility,
        marking_sharpness,
    }
}

fn calculate_fin_build(streak_week_count: usize) -> f64 {
    (streak_week_count as f64 / STREAK_FIN_FULL_WEEKS).min(1.0)
}

/// Remaining share after a streak ends: a negative exponential shifted and scaled
/// so it starts at one and reaches zero at FADE_WEEKS.
fn calculate_fade(weeks_since_end: f64) -> f64 {
    let fade_floor = (-FADE_RATE_PER_WEEK * FADE_WEEKS).exp();
    (((-FADE_RATE_PER_WEEK * weeks_since_end).exp() - fade_floor) / (1.0 - fade_floor)).max(0.0)
}

fn calculate_record_strength(longest_weeks: f64) -> f64 {
    ((1.0 + longest_weeks / RECORD_CURVE_WEEKS).ln()
        / (1.0 + FULL_GLIMMER_WEEKS / RECORD_CURVE_WEEKS).ln())
    .min(1.0)
}
