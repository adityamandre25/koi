use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};
use git_koi::history::{
    AccountSnapshot, ConflictingRepository, ContributionDay, ContributionHistory,
    ContributionHistoryError, LanguageColor,
};
use git_koi::koi::{KoiDevelopmentError, RecentActivity, RecentStreak, derive_koi_development};

#[test]
fn identity_survives_growth_and_language_changes_but_distinguishes_accounts() {
    let mut account_snapshot = build_snapshot_with_counts(&[1; 7]);
    let original_development = derive_koi_development(&account_snapshot).unwrap();
    account_snapshot.repositories.clear();
    account_snapshot.contribution_history.as_mut().unwrap().days[0].contribution_count += 5;
    let changed_development = derive_koi_development(&account_snapshot).unwrap();
    assert_eq!(
        changed_development.identity_seed,
        original_development.identity_seed
    );
    assert_ne!(
        changed_development.language_shares,
        original_development.language_shares
    );
    assert_eq!(
        changed_development.total_contributions,
        original_development.total_contributions + 5
    );
    account_snapshot.account_id += 1;
    assert_ne!(
        derive_koi_development(&account_snapshot)
            .unwrap()
            .identity_seed,
        original_development.identity_seed
    );
}

#[test]
fn development_is_reproducible_after_reordering_and_duplicate_observations() {
    let mut account_snapshot = build_snapshot_with_counts(&[1; 15]);
    let original_development = derive_koi_development(&account_snapshot).unwrap();
    account_snapshot
        .repositories
        .push(account_snapshot.repositories[0].clone());
    account_snapshot.repositories.reverse();
    let contribution_days = &mut account_snapshot.contribution_history.as_mut().unwrap().days;
    contribution_days.push(contribution_days[0].clone());
    contribution_days.reverse();
    assert_eq!(
        derive_koi_development(&account_snapshot).unwrap(),
        original_development
    );
}

#[test]
fn twelve_week_streaks_retain_different_participation_with_equal_totals() {
    for (weekly_counts, expected_active_days) in
        [([1, 1, 1, 1, 1, 0, 0], 5), ([0, 0, 0, 0, 0, 5, 0], 1)]
    {
        let account_snapshot = build_snapshot_with_counts(&weekly_counts.repeat(12));
        let koi_development = derive_koi_development(&account_snapshot).unwrap();
        assert_eq!(koi_development.total_contributions, 60);
        assert_eq!(
            koi_development
                .streaks
                .longest
                .iter()
                .map(|contribution_week| contribution_week.active_day_count)
                .collect::<Vec<_>>(),
            vec![expected_active_days; 12]
        );
        assert_eq!(
            koi_development.streaks.recent,
            Some(RecentStreak::Current(koi_development.streaks.longest))
        );
    }
}

#[test]
fn inactive_weeks_end_the_current_streak_without_erasing_or_restarting_it() {
    let mut daily_counts = vec![1; 7];
    daily_counts.extend([0; 7]);
    let unfinished_week_development =
        derive_koi_development(&build_snapshot_with_counts(&daily_counts)).unwrap();
    assert_eq!(
        unfinished_week_development.streaks.recent,
        Some(RecentStreak::Current(
            unfinished_week_development.streaks.longest.clone()
        ))
    );
    daily_counts.push(0);
    let ended_development =
        derive_koi_development(&build_snapshot_with_counts(&daily_counts)).unwrap();
    assert_eq!(
        ended_development.streaks.longest,
        unfinished_week_development.streaks.longest
    );
    assert_eq!(
        ended_development.streaks.recent,
        Some(RecentStreak::Ended {
            weeks: unfinished_week_development.streaks.longest,
            ended_on: "2025-12-29".parse().unwrap(),
        })
    );
    daily_counts.extend([0; 14]);
    let quiet_development =
        derive_koi_development(&build_snapshot_with_counts(&daily_counts)).unwrap();
    assert_eq!(quiet_development.streaks, ended_development.streaks);
    assert_eq!(
        quiet_development.total_contributions,
        ended_development.total_contributions
    );
}

#[test]
fn returning_streak_changes_the_record_only_after_strictly_beating_it() {
    let mut daily_counts = vec![1; 14];
    daily_counts.extend([0; 7]);
    daily_counts.push(1);
    for (expected_current_weeks, expected_record_days) in
        [(1, &[7, 7][..]), (2, &[7, 7][..]), (3, &[1, 1, 1][..])]
    {
        let koi_development =
            derive_koi_development(&build_snapshot_with_counts(&daily_counts)).unwrap();
        let Some(RecentStreak::Current(current_weeks)) = koi_development.streaks.recent else {
            panic!("returning activity must produce a current streak");
        };
        assert_eq!(current_weeks.len(), expected_current_weeks);
        assert_eq!(
            koi_development
                .streaks
                .longest
                .iter()
                .map(|contribution_week| contribution_week.active_day_count)
                .collect::<Vec<_>>(),
            expected_record_days
        );
        daily_counts.extend([0; 6]);
        daily_counts.push(1);
    }
}

#[test]
fn a_shorter_ended_streak_remains_available_alongside_the_permanent_record() {
    let mut daily_counts = vec![1; 14];
    daily_counts.extend([0; 7]);
    daily_counts.push(1);
    daily_counts.extend([0; 14]);
    let koi_development =
        derive_koi_development(&build_snapshot_with_counts(&daily_counts)).unwrap();
    assert_eq!(koi_development.streaks.longest.len(), 2);
    let Some(RecentStreak::Ended {
        weeks: ended_weeks,
        ended_on,
    }) = koi_development.streaks.recent
    else {
        panic!("the latest shorter streak must remain available for fading");
    };
    assert_eq!(ended_weeks.len(), 1);
    assert_eq!(ended_weeks[0].active_day_count, 1);
    assert_eq!(ended_on, "2026-01-19".parse::<NaiveDate>().unwrap());
}

#[test]
fn empty_observations_produce_no_streaks_but_unavailable_history_fails() {
    let mut account_snapshot = build_snapshot_with_counts(&[0; 14]);
    let inactive_development = derive_koi_development(&account_snapshot).unwrap();
    assert_eq!(inactive_development.total_contributions, 0);
    assert!(inactive_development.streaks.longest.is_empty());
    assert!(inactive_development.streaks.recent.is_none());
    account_snapshot
        .contribution_history
        .as_mut()
        .unwrap()
        .days
        .clear();
    // Observed quiet days are recent activity; an empty history has none to report.
    let empty_development = derive_koi_development(&account_snapshot).unwrap();
    assert_eq!(empty_development.total_contributions, 0);
    assert_eq!(empty_development.streaks, inactive_development.streaks);
    account_snapshot.contribution_history = None;
    assert_eq!(
        derive_koi_development(&account_snapshot),
        Err(KoiDevelopmentError::ContributionHistory(
            ContributionHistoryError::MissingHistory
        ))
    );
}

#[test]
fn conflicting_repositories_cannot_produce_a_partial_development_model() {
    let mut account_snapshot = build_snapshot_with_counts(&[1]);
    let mut conflicting_repository = account_snapshot.repositories[0].clone();
    conflicting_repository.is_fork = !conflicting_repository.is_fork;
    let expected_error = ConflictingRepository {
        repository_id: conflicting_repository.repository_id,
    };
    account_snapshot.repositories.push(conflicting_repository);
    assert_eq!(
        derive_koi_development(&account_snapshot),
        Err(KoiDevelopmentError::Repositories(expected_error))
    );
}

#[test]
fn development_carries_the_color_of_each_counted_language() {
    let mut account_snapshot = build_snapshot_with_counts(&[1; 7]);
    let recorded_colors = account_snapshot.language_colors.as_mut().unwrap();
    recorded_colors.insert("C++".to_owned(), None);
    recorded_colors.insert("Go".to_owned(), Some(LanguageColor([0x00, 0xad, 0xd8])));
    assert_eq!(
        derive_koi_development(&account_snapshot)
            .unwrap()
            .language_colors,
        BTreeMap::from([
            ("C++".to_owned(), None),
            ("Python".to_owned(), Some(LanguageColor([0x35, 0x72, 0xa5]))),
            ("Rust".to_owned(), Some(LanguageColor([0xde, 0xa5, 0x84]))),
        ])
    );
}

#[test]
fn snapshots_without_language_colors_ask_for_a_new_collection() {
    let mut account_snapshot = build_snapshot_with_counts(&[1; 7]);
    account_snapshot
        .language_colors
        .as_mut()
        .unwrap()
        .remove("Python");
    assert_eq!(
        derive_koi_development(&account_snapshot),
        Err(KoiDevelopmentError::MissingLanguageColor(
            "Python".to_owned()
        ))
    );
    account_snapshot.language_colors = None;
    let missing_colors = derive_koi_development(&account_snapshot);
    assert_eq!(
        missing_colors,
        Err(KoiDevelopmentError::MissingLanguageColors)
    );
    assert!(
        missing_colors
            .unwrap_err()
            .to_string()
            .contains("git-koi collect")
    );
}

#[test]
fn recent_activity_counts_quiet_days_the_last_week_and_complete_weeks() {
    let activity_for = |daily_counts: &[u32]| {
        derive_koi_development(&build_snapshot_with_counts(daily_counts))
            .unwrap()
            .recent_activity
    };
    // Two full weeks, then a quiet Monday and Tuesday.
    assert_eq!(
        activity_for(&[vec![1; 7], vec![0, 0, 0, 0, 0, 0, 2], vec![0, 0]].concat()),
        RecentActivity {
            quiet_day_count: 2,
            recent_contributions: 2,
            complete_week_contributions: vec![7, 2],
            observation_week_contributions: vec![0, 0],
        }
    );
    // Active on the observation date, partway through the first week.
    assert_eq!(
        activity_for(&[0, 0, 3]),
        RecentActivity {
            quiet_day_count: 0,
            recent_contributions: 3,
            complete_week_contributions: Vec::new(),
            observation_week_contributions: vec![0, 0, 3],
        }
    );
    // Never active: every observed day is quiet.
    assert_eq!(activity_for(&[0; 5]).quiet_day_count, 5);
    // Observed on a Sunday, the full observation week is still in progress.
    let sunday_activity = activity_for(&[1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(
        sunday_activity.complete_week_contributions,
        Vec::<u64>::new()
    );
    assert_eq!(
        sunday_activity.observation_week_contributions,
        vec![1, 2, 3, 4, 5, 6, 7]
    );
    let mut empty_snapshot = build_snapshot_with_counts(&[1]);
    empty_snapshot
        .contribution_history
        .as_mut()
        .unwrap()
        .days
        .clear();
    assert_eq!(
        derive_koi_development(&empty_snapshot)
            .unwrap()
            .recent_activity,
        RecentActivity {
            quiet_day_count: 0,
            recent_contributions: 0,
            complete_week_contributions: Vec::new(),
            observation_week_contributions: Vec::new(),
        }
    );
}

#[test]
fn lifetime_growth_milestones_and_smooth_logarithmic_interpolation() {
    use git_koi::koi::{
        COLOR_SATURATION_MILESTONE, DEVELOPED_FINS_MILESTONE, JUVENILE_TO_ADULT_MILESTONE,
        LEGENDARY_MILESTONE, MATURE_SHOW_MILESTONE, calculate_growth_fraction,
    };

    assert_eq!(calculate_growth_fraction(0), 0.0);
    assert!((calculate_growth_fraction(COLOR_SATURATION_MILESTONE) - 0.20).abs() < 1e-9);
    assert!((calculate_growth_fraction(JUVENILE_TO_ADULT_MILESTONE) - 0.40).abs() < 1e-9);
    assert!((calculate_growth_fraction(DEVELOPED_FINS_MILESTONE) - 0.65).abs() < 1e-9);
    assert!((calculate_growth_fraction(MATURE_SHOW_MILESTONE) - 0.85).abs() < 1e-9);
    assert_eq!(calculate_growth_fraction(LEGENDARY_MILESTONE), 1.0);
    assert_eq!(calculate_growth_fraction(50_000), 1.0);

    // Monotonicity check across the entire spectrum
    let mut prev_growth = 0.0;
    for c in [1, 10, 25, 35, 50, 100, 150, 250, 400, 700, 1000, 2000] {
        let g = calculate_growth_fraction(c);
        assert!(
            g >= prev_growth,
            "Growth must never decrease: at {c}, {g} < {prev_growth}"
        );
        prev_growth = g;
    }
}

#[test]
fn marking_streak_breaks_on_zero_commit_day_and_persists_across_weeks() {
    // 2026-11-02 was Monday.
    // Mon: 1, Tue: 1, Wed: 0 (missed! streak breaks), Thu: 1, Fri: 1, Sat: 1, Sun: 10 (Sunday ignored)
    let daily_counts = [1, 1, 0, 1, 1, 1, 10];
    let snapshot = build_snapshot_with_start_date("2026-11-02", &daily_counts);
    let dev = derive_koi_development(&snapshot).unwrap();

    // Thu, Fri, Sat are 3 active streak days. Sunday does not count.
    assert_eq!(dev.weekly_active_days, 3);
    assert_eq!(dev.consecutive_active_days, 3);

    // Full 6-day Mon-Sat streak
    let full_streak = [1, 1, 1, 1, 1, 1, 0];
    let full_snapshot = build_snapshot_with_start_date("2026-11-02", &full_streak);
    let full_dev = derive_koi_development(&full_snapshot).unwrap();
    assert_eq!(full_dev.consecutive_active_days, 6);
    assert_eq!(full_dev.weekly_active_days, 6);

    // Sunday active also does not increment streak beyond Saturday
    let full_with_sunday = [1, 1, 1, 1, 1, 1, 10];
    let full_sun_snapshot = build_snapshot_with_start_date("2026-11-02", &full_with_sunday);
    let full_sun_dev = derive_koi_development(&full_sun_snapshot).unwrap();
    assert_eq!(full_sun_dev.consecutive_active_days, 6);
    assert_eq!(full_sun_dev.weekly_active_days, 6);

    // Next Monday has 1 commit: streak does NOT reset every week, it persists across weeks!
    let next_monday_active = [1, 1, 1, 1, 1, 1, 10, 1];
    let next_active_snapshot = build_snapshot_with_start_date("2026-11-02", &next_monday_active);
    let next_active_dev = derive_koi_development(&next_active_snapshot).unwrap();
    assert_eq!(next_active_dev.consecutive_active_days, 7);
    assert_eq!(next_active_dev.weekly_active_days, 7);

    // When 0 commits occur on a Mon-Sat day after days of commits, streak breaks and resets to 0!
    let zero_commit_break = [1, 1, 1, 1, 1, 1, 10, 0];
    let break_snapshot = build_snapshot_with_start_date("2026-11-02", &zero_commit_break);
    let break_dev = derive_koi_development(&break_snapshot).unwrap();
    assert_eq!(break_dev.consecutive_active_days, 0);
    assert_eq!(break_dev.weekly_active_days, 0);
}

#[test]
fn current_year_contributions_counts_from_january_first_and_resets() {
    // Days across 2025 to 2026:
    // 2025-12-30: 5
    // 2025-12-31: 5
    // 2026-01-01: 3
    // 2026-01-02: 4
    let cross_year_counts = [5, 5, 3, 4];
    let snapshot = build_snapshot_with_start_date("2025-12-30", &cross_year_counts);
    let dev = derive_koi_development(&snapshot).unwrap();

    // Total lifetime: 5 + 5 + 3 + 4 = 17
    assert_eq!(dev.total_contributions, 17);
    // Current year (2026): only 2026-01-01 (3) + 2026-01-02 (4) = 7
    assert_eq!(dev.current_year_contributions, 7);
}

fn build_snapshot_with_start_date(start_date_str: &str, daily_counts: &[u32]) -> AccountSnapshot {
    let start_date: NaiveDate = start_date_str.parse().unwrap();
    let contribution_days: Vec<_> = daily_counts
        .iter()
        .enumerate()
        .map(|(offset, &count)| ContributionDay {
            date: start_date
                .checked_add_days(Days::new(offset as u64))
                .unwrap(),
            contribution_count: count,
        })
        .collect();
    let mut account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account.json")).unwrap();
    account_snapshot.contribution_history = Some(ContributionHistory {
        observed_on: contribution_days.last().unwrap().date,
        days: contribution_days,
    });
    account_snapshot.language_colors = Some(BTreeMap::from([
        ("C++".to_owned(), Some(LanguageColor([0xf3, 0x4b, 0x7d]))),
        ("Python".to_owned(), Some(LanguageColor([0x35, 0x72, 0xa5]))),
        ("Rust".to_owned(), Some(LanguageColor([0xde, 0xa5, 0x84]))),
    ]));
    account_snapshot
}

fn build_snapshot_with_counts(daily_counts: &[u32]) -> AccountSnapshot {
    let first_monday: NaiveDate = "2025-12-15".parse().unwrap();
    let contribution_days: Vec<_> = daily_counts
        .iter()
        .enumerate()
        .map(|(day_offset, contribution_count)| ContributionDay {
            date: first_monday
                .checked_add_days(Days::new(day_offset as u64))
                .unwrap(),
            contribution_count: *contribution_count,
        })
        .collect();
    let mut account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account.json")).unwrap();
    account_snapshot.contribution_history = Some(ContributionHistory {
        observed_on: contribution_days.last().unwrap().date,
        days: contribution_days,
    });
    account_snapshot.language_colors = Some(BTreeMap::from([
        ("C++".to_owned(), Some(LanguageColor([0xf3, 0x4b, 0x7d]))),
        ("Python".to_owned(), Some(LanguageColor([0x35, 0x72, 0xa5]))),
        ("Rust".to_owned(), Some(LanguageColor([0xde, 0xa5, 0x84]))),
    ]));
    account_snapshot
}
