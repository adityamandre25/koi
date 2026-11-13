//! Reproducible koi development from account observations, independent of rendering.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use chrono::{Datelike, Days, IsoWeek, NaiveDate, Weekday};

use crate::history::{
    AccountSnapshot, ConflictingRepository, ContributionHistoryError, ContributionSummary,
    ContributionWeek, LanguageColor, calculate_language_shares, summarize_contributions,
};

/// Derived inputs for appearance; source observations remain in the account snapshot.
#[derive(Debug, PartialEq)]
pub struct KoiDevelopment {
    /// The stable numeric GitHub account ID.
    pub identity_seed: u64,
    /// Each counted language's share of eligible repositories, from zero to one, by name.
    pub language_shares: BTreeMap<String, f64>,
    /// GitHub's color for each language in `language_shares`; `None` means it has no color.
    pub language_colors: BTreeMap<String, Option<LanguageColor>>,
    /// Unweighted contributions available to the renderer's growth curve.
    pub total_contributions: u64,
    /// The date every time-based measurement is taken against.
    pub observed_on: NaiveDate,
    /// The longest and the most recent weekly streaks.
    pub streaks: StreakDevelopment,
    /// Counts around the observation date that behavior selection reads.
    pub recent_activity: RecentActivity,
    /// Active days Monday-Saturday only in the observation week (0..=6). Sunday excluded.
    pub weekly_active_days: u8,
    /// Consecutive active days Monday-Saturday in the observation week (0..=6). Sunday ignored.
    pub consecutive_active_days: u8,
    /// Yesterday's contribution count (on observed_on), including Sunday.
    pub yesterday_contributions: u32,
    /// Current year contributions from Jan 1 through observed_on, including Sunday.
    pub current_year_contributions: u64,
}

/// Recent activity as of the observation date, as counts rather than judgments.
#[derive(Debug, Eq, PartialEq)]
pub struct RecentActivity {
    /// Days in the run of contribution-free days that ends on the observation date;
    /// an account that has never contributed counts every observed day.
    pub quiet_day_count: u32,
    /// Contributions in the seven days ending on the observation date.
    pub recent_contributions: u64,
    /// Contributions in each complete Monday-Sunday week, oldest first. The
    /// observation week is still in progress and is left out.
    pub complete_week_contributions: Vec<u64>,
    /// Each observed day of the observation week, from Monday through the
    /// observation date.
    pub observation_week_contributions: Vec<u64>,
}

/// Weekly streaks: runs of consecutive Monday-Sunday weeks with at least one active day.
#[derive(Debug, Eq, PartialEq)]
pub struct StreakDevelopment {
    /// The earliest longest run, replaced only by a strictly longer run.
    pub longest: Vec<ContributionWeek>,
    /// None means no observed active week, not missing history.
    pub recent: Option<RecentStreak>,
}

/// The most recent streak, still running or ended.
#[derive(Debug, Eq, PartialEq)]
pub enum RecentStreak {
    /// Still running through the observation week; carries its weeks, oldest first.
    Current(Vec<ContributionWeek>),
    /// Ended by a completed week without activity.
    Ended {
        /// The streak's weeks, oldest first.
        weeks: Vec<ContributionWeek>,
        /// The Monday after the first wholly inactive week has completed.
        /// Further inactivity does not move this date or restart a fade.
        ended_on: NaiveDate,
    },
}

/// Derive one account's identity, language composition, and contribution development.
///
/// Any active day sustains a Monday-Sunday weekly streak. A completed inactive
/// week ends it; the observation week is still in progress and cannot end it.
/// Each streak retains weekly participation rather than just its length.
/// Returning activity replaces the recent streak without discarding the record.
///
/// History owns input validation and language eligibility. Its errors propagate;
/// unavailable history never becomes an undeveloped koi. Confirmed empty history
/// produces zero contributions and no streaks. Every counted language needs a
/// recorded color, so a snapshot without colors fails instead of producing an
/// uncolored koi. Inputs are borrowed without mutation; this operation performs
/// no I/O and reads no clock. It assigns no visual thresholds.
pub fn derive_koi_development(
    account_snapshot: &AccountSnapshot,
) -> Result<KoiDevelopment, KoiDevelopmentError> {
    let contribution_summary = summarize_contributions(account_snapshot)
        .map_err(KoiDevelopmentError::ContributionHistory)?;
    let language_shares =
        calculate_language_shares(account_snapshot).map_err(KoiDevelopmentError::Repositories)?;
    let recorded_colors = account_snapshot
        .language_colors
        .as_ref()
        .ok_or(KoiDevelopmentError::MissingLanguageColors)?;
    let language_colors = language_shares
        .keys()
        .map(|language_name| {
            recorded_colors
                .get(language_name)
                .map(|language_color| (language_name.clone(), *language_color))
                .ok_or_else(|| KoiDevelopmentError::MissingLanguageColor(language_name.clone()))
        })
        .collect::<Result<_, _>>()?;
    let observed_on = contribution_summary.observed_on;
    let daily_counts = &contribution_summary.daily_counts;
    let weekly_active_days = calculate_weekly_active_days(daily_counts, observed_on);
    let consecutive_active_days = calculate_consecutive_active_days(daily_counts, observed_on);
    let yesterday_contributions = daily_counts.get(&observed_on).copied().unwrap_or(0);
    let year_start = NaiveDate::from_ymd_opt(observed_on.year(), 1, 1)
        .expect("valid start date for observation year");
    let current_year_contributions = daily_counts
        .range(year_start..=observed_on)
        .map(|(_, &count)| u64::from(count))
        .sum();

    Ok(KoiDevelopment {
        identity_seed: account_snapshot.account_id,
        language_shares,
        language_colors,
        total_contributions: contribution_summary.total_contributions,
        observed_on,
        streaks: calculate_streak_development(&contribution_summary),
        recent_activity: measure_recent_activity(&contribution_summary),
        weekly_active_days,
        consecutive_active_days,
        yesterday_contributions,
        current_year_contributions,
    })
}

/// 0–25: gradual colour saturation milestone.
pub const COLOR_SATURATION_MILESTONE: u64 = 25;
/// 50: juvenile-to-adult anatomical transformation milestone.
pub const JUVENILE_TO_ADULT_MILESTONE: u64 = 50;
/// 150: broader and longer body, developed fins milestone.
pub const DEVELOPED_FINS_MILESTONE: u64 = 150;
/// 400: mature show koi with flowing veil-tail fins milestone.
pub const MATURE_SHOW_MILESTONE: u64 = 400;
/// 1,000: legendary maximum development milestone.
pub const LEGENDARY_MILESTONE: u64 = 1_000;

/// Compatibility aliases with earlier constants.
/// Young shape milestone alias.
pub const YOUNG_SHAPE_CONTRIBUTIONS: u64 = JUVENILE_TO_ADULT_MILESTONE;
/// Adult contributions milestone alias.
pub const ADULT_CONTRIBUTIONS: u64 = MATURE_SHOW_MILESTONE;
/// Maximum growth contributions milestone alias.
pub const MAX_GROWTH_CONTRIBUTIONS: u64 = LEGENDARY_MILESTONE;
/// Full color saturation milestone alias.
pub const FULL_COLOR_CONTRIBUTIONS: u64 = COLOR_SATURATION_MILESTONE;

/// Smooth logarithmic interpolation across a range [start, end].
/// Returns 0.0 at or below `start`, 1.0 at or above `end`, and logarithmic progress between them.
pub fn log_interpolate(value: u64, start: u64, end: u64) -> f64 {
    if value <= start {
        0.0
    } else if value >= end {
        1.0
    } else {
        let delta = (value - start) as f64;
        let range = (end - start) as f64;
        (1.0 + delta).ln() / (1.0 + range).ln()
    }
}

/// Map total contributions to a permanent physical growth fraction from 0.0 through 1.0
/// using smooth logarithmic interpolation between milestones:
/// - 0..=25: color saturation, juvenile form (0.00 -> 0.20)
/// - 25..=50: juvenile-to-adult anatomical transformation (0.20 -> 0.40)
/// - 50..=150: broader and longer body, developed fins (0.40 -> 0.65)
/// - 150..=400: mature show koi with flowing veil-tail fins (0.65 -> 0.85)
/// - 400..=1,000: legendary maximum development (0.85 -> 1.00)
///
/// Totals above 1,000 keep the legendary maximum. Growth never decreases due to inactivity.
pub fn calculate_growth_fraction(total_contributions: u64) -> f64 {
    if total_contributions <= COLOR_SATURATION_MILESTONE {
        0.20 * log_interpolate(total_contributions, 0, COLOR_SATURATION_MILESTONE)
    } else if total_contributions <= JUVENILE_TO_ADULT_MILESTONE {
        0.20 + 0.20
            * log_interpolate(
                total_contributions,
                COLOR_SATURATION_MILESTONE,
                JUVENILE_TO_ADULT_MILESTONE,
            )
    } else if total_contributions <= DEVELOPED_FINS_MILESTONE {
        0.40 + 0.25
            * log_interpolate(
                total_contributions,
                JUVENILE_TO_ADULT_MILESTONE,
                DEVELOPED_FINS_MILESTONE,
            )
    } else if total_contributions <= MATURE_SHOW_MILESTONE {
        0.65 + 0.20
            * log_interpolate(
                total_contributions,
                DEVELOPED_FINS_MILESTONE,
                MATURE_SHOW_MILESTONE,
            )
    } else if total_contributions <= LEGENDARY_MILESTONE {
        0.85 + 0.15
            * log_interpolate(
                total_contributions,
                MATURE_SHOW_MILESTONE,
                LEGENDARY_MILESTONE,
            )
    } else {
        1.0
    }
}

/// 0–25: gradual colour saturation progress from 0.0 (baseline juvenile pale) to 1.0 (full saturation).
pub fn calculate_color_saturation_progress(total_contributions: u64) -> f64 {
    log_interpolate(total_contributions, 0, COLOR_SATURATION_MILESTONE)
}

/// 0–50: juvenile-to-adult anatomical transformation progress from 0.0 (juvenile) to 1.0 (adult anatomy).
pub fn calculate_juvenile_adult_transformation(total_contributions: u64) -> f64 {
    log_interpolate(total_contributions, 0, JUVENILE_TO_ADULT_MILESTONE)
}

/// 50–150: broader and longer body, developed fins progress.
pub fn calculate_developed_fins_transformation(total_contributions: u64) -> f64 {
    log_interpolate(
        total_contributions,
        JUVENILE_TO_ADULT_MILESTONE,
        DEVELOPED_FINS_MILESTONE,
    )
}

/// 150–400: mature show koi with flowing veil-tail fins progress.
pub fn calculate_veil_tail_transformation(total_contributions: u64) -> f64 {
    log_interpolate(
        total_contributions,
        DEVELOPED_FINS_MILESTONE,
        MATURE_SHOW_MILESTONE,
    )
}

/// 400–1,000: legendary maximum development progress.
pub fn calculate_legendary_transformation(total_contributions: u64) -> f64 {
    log_interpolate(
        total_contributions,
        MATURE_SHOW_MILESTONE,
        LEGENDARY_MILESTONE,
    )
}

/// Count active days in the current unbroken streak (Monday through Saturday).
/// - Resets when the streak breaks (i.e. 0 commits on a Monday–Saturday day).
/// - Does NOT reset on Monday / week boundaries; persists across weeks.
/// - Sunday is completely ignored: it neither increments nor breaks the streak.
pub fn calculate_streak_active_days(
    daily_counts: &BTreeMap<NaiveDate, u32>,
    observed_on: NaiveDate,
) -> u8 {
    let mut current_streak: u8 = 0;
    let mut date = observed_on;

    // If observed_on is Sunday, Sunday is completely ignored: it neither increments nor breaks the streak.
    // Evaluate the streak as of the latest Saturday.
    if date.weekday() == Weekday::Sun {
        let Some(prev) = date.pred_opt() else {
            return 0;
        };
        date = prev;
    }

    // Walk backwards day by day from date
    while let Some(&count) = daily_counts.get(&date) {
        if date.weekday() == Weekday::Sun {
            // Sunday is completely ignored: neither increments nor breaks streak.
        } else if count > 0 {
            current_streak = current_streak.saturating_add(1);
        } else {
            // 0 commits on a Monday–Saturday day: streak broke!
            break;
        }

        let Some(prev_date) = date.pred_opt() else {
            break;
        };
        date = prev_date;
    }

    current_streak
}

/// Calculate marking visibility active streak days (Monday through Saturday).
/// Resets when the streak breaks (0 commits on a Monday-Saturday day), does NOT reset every week.
pub fn calculate_weekly_active_days(
    daily_counts: &BTreeMap<NaiveDate, u32>,
    observed_on: NaiveDate,
) -> u8 {
    calculate_streak_active_days(daily_counts, observed_on)
}

/// Calculate marking sharpness consecutive active days (Monday through Saturday).
/// Resets when the streak breaks (0 commits on a Monday-Saturday day), does NOT reset every week.
pub fn calculate_consecutive_active_days(
    daily_counts: &BTreeMap<NaiveDate, u32>,
    observed_on: NaiveDate,
) -> u8 {
    calculate_streak_active_days(daily_counts, observed_on)
}

/// Why a snapshot produced no development. Each message names the repair.
#[derive(Debug, Eq, PartialEq)]
pub enum KoiDevelopmentError {
    /// The contribution history failed its rules.
    ContributionHistory(ContributionHistoryError),
    /// Two observations of one repository disagree.
    Repositories(ConflictingRepository),
    /// The snapshot predates language colors.
    MissingLanguageColors,
    /// The snapshot has no color entry for this counted language.
    MissingLanguageColor(String),
}

impl fmt::Display for KoiDevelopmentError {
    fn fmt(&self, output_formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContributionHistory(history_error) => write!(output_formatter, "{history_error}"),
            Self::Repositories(repository_error) => write!(output_formatter, "{repository_error}"),
            Self::MissingLanguageColors => write!(
                output_formatter,
                "the snapshot has no language colors; collect it again with `git-koi collect`"
            ),
            Self::MissingLanguageColor(language_name) => write!(
                output_formatter,
                "the snapshot has no color for {language_name}; collect it again with `git-koi collect`"
            ),
        }
    }
}

impl Error for KoiDevelopmentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ContributionHistory(history_error) => Some(history_error),
            Self::Repositories(repository_error) => Some(repository_error),
            Self::MissingLanguageColors | Self::MissingLanguageColor(_) => None,
        }
    }
}

fn measure_recent_activity(contribution_summary: &ContributionSummary) -> RecentActivity {
    const RECENT_DAY_COUNT: u64 = 7;
    const DAYS_PER_WEEK: u64 = 7;
    let observed_on = contribution_summary.observed_on;
    let daily_counts = &contribution_summary.daily_counts;
    let quiet_day_count = daily_counts
        .values()
        .rev()
        .take_while(|&&contribution_count| contribution_count == 0)
        .count() as u32;
    let recent_start = observed_on
        .checked_sub_days(Days::new(RECENT_DAY_COUNT - 1))
        .expect("observation dates are far from the calendar's start");
    let recent_contributions = daily_counts
        .range(recent_start..=observed_on)
        .map(|(_, &contribution_count)| u64::from(contribution_count))
        .sum();
    let mut weekly_totals: BTreeMap<IsoWeek, (u64, u64)> = BTreeMap::new();
    for (contribution_date, &contribution_count) in daily_counts {
        let (week_contributions, week_day_count) = weekly_totals
            .entry(contribution_date.iso_week())
            .or_default();
        *week_contributions += u64::from(contribution_count);
        *week_day_count += 1;
    }
    let observation_week = observed_on.iso_week();
    let observation_monday = observed_on
        .checked_sub_days(Days::new(u64::from(
            observed_on.weekday().num_days_from_monday(),
        )))
        .expect("observation dates are far from the calendar's start");
    RecentActivity {
        quiet_day_count,
        recent_contributions,
        complete_week_contributions: weekly_totals
            .into_iter()
            .filter(|&(iso_week, (_, week_day_count))| {
                week_day_count == DAYS_PER_WEEK && iso_week != observation_week
            })
            .map(|(_, (week_contributions, _))| week_contributions)
            .collect(),
        observation_week_contributions: daily_counts
            .range(observation_monday..=observed_on)
            .map(|(_, &contribution_count)| u64::from(contribution_count))
            .collect(),
    }
}

fn calculate_streak_development(contribution_summary: &ContributionSummary) -> StreakDevelopment {
    let mut longest_week_range = 0..0;
    let mut recent_week_range = 0..0;
    let mut current_start_week_index = None;
    let mut ended_on = None;
    let observation_week = contribution_summary.observed_on.iso_week();
    for (week_index, contribution_week) in contribution_summary.weeks.iter().enumerate() {
        if contribution_week.active_day_count > 0 {
            let start_week_index = *current_start_week_index.get_or_insert(week_index);
            recent_week_range = start_week_index..week_index + 1;
            ended_on = None;
            if recent_week_range.len() > longest_week_range.len() {
                longest_week_range = recent_week_range.clone();
            }
        } else if contribution_week.iso_week != observation_week
            && current_start_week_index.take().is_some()
        {
            let inactive_sunday = NaiveDate::from_isoywd_opt(
                contribution_week.iso_week.year(),
                contribution_week.iso_week.week(),
                Weekday::Sun,
            )
            .expect("a completed observation week has a Sunday");
            ended_on = Some(
                inactive_sunday
                    .succ_opt()
                    .expect("a completed week precedes the observation date"),
            );
        }
    }
    let recent_streak = if recent_week_range.is_empty() {
        None
    } else {
        let recent_weeks = contribution_summary.weeks[recent_week_range].to_vec();
        Some(match ended_on {
            Some(ended_on) => RecentStreak::Ended {
                weeks: recent_weeks,
                ended_on,
            },
            None => RecentStreak::Current(recent_weeks),
        })
    };
    StreakDevelopment {
        longest: contribution_summary.weeks[longest_week_range].to_vec(),
        recent: recent_streak,
    }
}
