use chrono::{Days, NaiveDate};
use git_koi::behavior::{Behavior, select_behavior};
use git_koi::history::{AccountSnapshot, ContributionDay, ContributionHistory};
use git_koi::koi::{KoiDevelopment, derive_koi_development};

#[test]
fn yesterday_zero_contributions_selects_peaceful_gliding() {
    let koi = develop_with_yesterday_contributions(0);
    assert_eq!(select_behavior(&koi), Behavior::PeacefulGliding);
}

#[test]
fn yesterday_one_to_three_contributions_selects_gentle_swimming() {
    for count in 1..=3 {
        let koi = develop_with_yesterday_contributions(count);
        assert_eq!(select_behavior(&koi), Behavior::GentleSwimming, "{count} contributions");
    }
}

#[test]
fn yesterday_four_to_ten_contributions_selects_energetic_swimming() {
    for count in [4, 5, 8, 10] {
        let koi = develop_with_yesterday_contributions(count);
        assert_eq!(select_behavior(&koi), Behavior::EnergeticSwimming, "{count} contributions");
    }
}

#[test]
fn yesterday_eleven_plus_contributions_selects_fast_swimming() {
    for count in [11, 15, 50, 200] {
        let koi = develop_with_yesterday_contributions(count);
        assert_eq!(select_behavior(&koi), Behavior::FastSwimming, "{count} contributions");
    }
}

#[test]
fn sunday_yesterday_activity_is_included() {
    // A Sunday observation date:
    // 2026-11-08 was a Sunday.
    let sunday = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
    assert_eq!(sunday.weekday(), chrono::Weekday::Sun);

    for (count, expected) in [
        (0, Behavior::PeacefulGliding),
        (2, Behavior::GentleSwimming),
        (6, Behavior::EnergeticSwimming),
        (14, Behavior::FastSwimming),
    ] {
        let koi = develop_with_date_and_yesterday(sunday, count);
        assert_eq!(select_behavior(&koi), expected, "Sunday with {count} contributions");
    }
}

#[test]
fn daily_swimming_activity_never_affects_permanent_growth() {
    // An account with 500 total contributions:
    // whether yesterday had 0, 3, 8, or 100 contributions,
    // the permanent growth fraction and anatomy remain identical.
    let koi_zero = develop_with_total_and_yesterday(500, 0);
    let koi_high = develop_with_total_and_yesterday(500, 50);

    assert_eq!(koi_zero.total_contributions, koi_high.total_contributions);
    assert_ne!(select_behavior(&koi_zero), select_behavior(&koi_high));
}

fn develop_with_yesterday_contributions(yesterday_count: u32) -> KoiDevelopment {
    let observation_date = NaiveDate::from_ymd_opt(2026, 11, 12).unwrap();
    develop_with_date_and_yesterday(observation_date, yesterday_count)
}

fn develop_with_date_and_yesterday(observed_on: NaiveDate, yesterday_count: u32) -> KoiDevelopment {
    let start_date = observed_on.checked_sub_days(Days::new(14)).unwrap();
    let mut days = Vec::new();
    let mut date = start_date;
    while date < observed_on {
        days.push(ContributionDay {
            date,
            contribution_count: 1,
        });
        date = date.succ_opt().unwrap();
    }
    days.push(ContributionDay {
        date: observed_on,
        contribution_count: yesterday_count,
    });
    derive_koi(&days, observed_on)
}

fn develop_with_total_and_yesterday(total_contributions: u64, yesterday_count: u32) -> KoiDevelopment {
    let observed_on = NaiveDate::from_ymd_opt(2026, 11, 12).unwrap();
    let earlier_total = total_contributions.saturating_sub(u64::from(yesterday_count));
    let start_date = observed_on.checked_sub_days(Days::new(10)).unwrap();
    let mut days = Vec::new();
    let mut date = start_date;
    while date < observed_on {
        days.push(ContributionDay {
            date,
            contribution_count: (earlier_total / 10) as u32,
        });
        date = date.succ_opt().unwrap();
    }
    days.push(ContributionDay {
        date: observed_on,
        contribution_count: yesterday_count,
    });
    derive_koi(&days, observed_on)
}

fn derive_koi(days: &[ContributionDay], observed_on: NaiveDate) -> KoiDevelopment {
    let snapshot_json = format!(
        r#"{{
  "schema_version": 3,
  "account_id": 42,
  "repositories": [
    {{
      "repository_id": 1,
      "visibility": "public",
      "is_fork": false,
      "primary_language": "Rust"
    }}
  ],
  "contribution_history": {{
    "observed_on": "{observed_on}",
    "days": {}
  }},
  "language_colors": {{
    "Rust": "#dea584"
  }}
}}"#,
        serde_json::to_string(days).unwrap()
    );
    let snapshot: AccountSnapshot = serde_json::from_str(&snapshot_json).unwrap();
    derive_koi_development(&snapshot).unwrap()
}
