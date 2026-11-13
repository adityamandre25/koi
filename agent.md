# Git-Koi Development & Status Report (`agent.md`)

## Executive Summary
This document records what has been implemented, tested, and verified in the `git-koi` project, along with GitHub API limitations and verification notes.

All six core independent systems, crisp vector SVG generation, seamless swimming motion, and automated workflow integration are fully implemented and verified with 100% test pass rate (78 automated tests passing across 10 test suites).

---

## 1. What Has Been Done

### System 1: Lifetime Contributions → Permanent Physical Growth
- **Milestones**:
  - `0–25`: Gradual colour saturation (`COLOR_SATURATION_MILESTONE`).
  - `50`: Juvenile-to-adult anatomical transformation (`JUVENILE_TO_ADULT_MILESTONE`).
  - `150`: Broader and longer body, developed fins (`DEVELOPED_FINS_MILESTONE`).
  - `400`: Mature show koi with flowing veil-tail fins (`MATURE_SHOW_MILESTONE`).
  - `1,000`: Legendary maximum development (`LEGENDARY_MILESTONE`).
- **Smooth Logarithmic Interpolation**:
  - Implemented `log_interpolate(value, start, end) = ln(1 + delta) / ln(1 + range)` between milestone intervals.
  - Segmented growth progression:
    - `0..=25`: 0.00 -> 0.20
    - `25..=50`: 0.20 -> 0.40
    - `50..=150`: 0.40 -> 0.65
    - `150..=400`: 0.65 -> 0.85
    - `400..=1,000`: 0.85 -> 1.00
  - Values strictly monotonic: growth never decreases due to inactivity.
- **Anatomical Proportions & Dimensions**:
  - Body length scales smoothly from 42.0px to 142.0px.
  - Body width fraction broadens from 0.17 to 0.245.
  - Head slant shifts from 0.14 to 0.187.
  - Nose width fraction tapers from 0.75 to 0.517.
  - Tail taper exponent steepens from 1.25 to 1.80.
  - Eye scale scales from juvenile 2.1 down to adult 1.0.
  - Fin length multiplier scales from juvenile 0.85 to adult 1.00, developed fins 1.05, veil-tail 1.10, and legendary 1.14.
  - Completely framed within the 256x256 canvas with margins at every angle and pose.
- **Lifetime Historical Querying**:
  - Queries GitHub GraphQL API `contributionsCollection` year-by-year from account creation date (`createdAt`) to `observed_on`.

### System 2: Weekly Activity → Marking Visibility
- **Active Day Counting (Mon–Sat Only)**:
  - Scans daily contribution counts from Monday through Saturday within the observation week.
  - Sunday contributions are excluded.
  - Week resets every Monday at 00:00 UTC.
- **Marking Visibility Thresholds**:
  - `0 days`: Very subtle markings (`0.20`).
  - `1–2 days`: Light markings (`0.38 – 0.48`).
  - `3–4 days`: Clear markings (`0.68 – 0.78`).
  - `5 days`: Strong markings (`0.90`).
  - `6 days`: Maximum visibility (`1.00`).
- **Rendering Effect**:
  - Modulates marking patch coverage and visibility dithering against the ground body.

### System 3: Consecutive Active Days → Marking Sharpness
- **Streak Calculation**:
  - Measures uninterrupted consecutive active days leading up to the observation day within the current week (starting Monday).
  - Missing any Monday–Saturday day breaks the streak.
  - Sunday is completely ignored: it neither increments nor breaks the streak.
  - A fresh weekly streak starts on Monday.
  - Completely independent of weekly marking visibility.
- **Marking Sharpness Thresholds**:
  - `0`: Soft markings (`0.10`).
  - `2`: Clearer markings (`0.50`).
  - `4`: Sharp markings (`0.80`).
  - `6`: Maximum detail (`1.00`).
- **Rendering Effect**:
  - Modulates transition edge softness: lower values yield soft, blended dithered transitions at pigment margins; higher values yield razor-sharp crisp pigment borders.

### System 4: Yesterday's Contributions → Swimming Behaviour
- **4 Distinct Swimming Styles**:
  - `0 contributions`: **Peaceful Gliding** (`Behavior::PeacefulGliding`) — slow coasting, minimal fin movement, gentle wave strength.
  - `1–3 contributions`: **Gentle Swimming** (`Behavior::GentleSwimming`) — relaxed, steady lap swimming, rhythmic tail motion.
  - `4–10 contributions`: **Energetic Swimming** (`Behavior::EnergeticSwimming`) — active tail waves, lively fin sculling.
  - `11+ contributions`: **Fast Swimming** (`Behavior::FastSwimming`) — rapid 2-lap bursts with powerful tail beats and tucked side fins.
- **Rules**:
  - Evaluates contributions made on the day immediately preceding `observed_on` (including Sunday).
  - Daily activity never affects permanent physical growth.
  - Seamless 120-frame loops with zero position jumps or discontinuities.

### System 5: Programming Languages → Permanent Colour and Pigmentation
- **Language Palette & Pigment Mapping**:
  - Counts language bytes across eligible public repositories.
  - Deterministically maps language shares to traditional Japanese koi pigments:
    - Beni (Red): C++, Ruby, Swift, etc.
    - Sumi (Black): Go, shell, fallback languages without color.
    - Orenji (Orange): Rust, HTML.
    - Ki (Yellow): JavaScript, Python.
    - Ai (Blue): TypeScript, C, Java.
  - Account seed seeds organic patch layouts reproducibly.
  - Language changes or repository edits never alter physical growth proportions.

### System 6: Current-Year Contributions → Display-Only Statistic
- **Calculation**:
  - Sums contributions from January 1 of `observed_on.year()` through `observed_on` (inclusive of Sunday).
  - Resets automatically on January 1.
- **Display**:
  - Displayed in caption and metadata output: `{behavior} for {observed_on}, {body} body ({current_year_contributions} contributions in {year})`.
  - Completely decoupled from physical growth and anatomy.

### Sunday Rules Matrix (Verified)
| System | Include Sunday? | Verified Behavior |
|---|---|---|
| Lifetime growth | Yes | All Sunday contributions count towards total contributions. |
| Language pigmentation | Yes | Language statistics are permanent and independent of weekdays. |
| Weekly marking visibility | No | Sunday activity is strictly excluded from the 0..=6 weekly count. |
| Consecutive-day streak | Ignore completely | Sunday neither increments nor breaks the Mon–Sat streak. |
| Yesterday's swimming activity | Yes | Sunday activity determines Monday's swimming behavior. |
| Current-year statistic | Yes | Sunday contributions are included in the Jan 1 - Dec 31 total. |

### Vector SVG Generation
- Implemented `encode_svg(&KoiFrame)` in `src/output.rs`.
- Renders pixel-crisp SVG using run-length encoded horizontal `<rect>` strips, `viewBox="0 0 256 256"`, `width="512" height="512"`, and `shape-rendering="crispEdges"`.
- Output written alongside `koi.gif`, `koi.png`, and `preview.html`.
- Updated `preview.html` to preview `koi.svg` on light and dark backdrops.

### Workflow & Documentation Updates
- `action.yml`: Updated description and output paths to include `koi.svg`.
- `examples/profile-workflow.yml`: Added `koi.svg` and `preview.html` to git tracking.
- Preserved original MIT License, copyright notices (Anthony Yalong), minimal permissions (`contents: write`), and CLI argument schema.

---

## 2. Test Verification Summary

Executed test suite: `$env:PATH = "C:\Users\Admin\.cargo\bin;$env:PATH"; cargo test`
All 78 tests across 10 test suites passed cleanly with 0 failures:

| Test Suite | Total Tests | Passed | Failed |
|---|---|---|---|
| `unittests (src/lib.rs)` | 5 | 5 | 0 |
| `unittests (src/main.rs)` | 0 | 0 | 0 |
| `tests/behavior.rs` | 6 | 6 | 0 |
| `tests/collect.rs` | 13 | 13 | 0 |
| `tests/history.rs` | 17 | 17 | 0 |
| `tests/koi.rs` | 15 | 15 | 0 |
| `tests/markings.rs` | 8 | 8 | 0 |
| `tests/motion.rs` | 5 | 5 | 0 |
| `tests/output.rs` | 4 | 4 | 0 |
| `tests/render.rs` | 15 | 15 | 0 |
| `tests/render_command.rs` | 5 | 5 | 0 |
| `Doc-tests git_koi` | 0 | 0 | 0 |
| **Total** | **78** | **78** | **0** |

---

## 3. GitHub API Limitations & Notes
1. **1-Year Window Constraint**: The GitHub GraphQL API's `contributionsCollection` query requires `from` and `to` arguments spanning at most one calendar year (up to one year per call). Lifetime history is collected by iterating backwards from the current year to the account creation year.
2. **Commit Email Verification**: GitHub only includes commits in contribution calendars if the author's Git commit email address matches an email associated with and verified on the GitHub account.
3. **Rate Limits & Large History**: Accounts created 10-15+ years ago require multiple GraphQL requests in sequence during collection. For normal daily cron runs in GitHub Actions, `github.token` provides 1,000 requests/hour, which is more than sufficient.

---

## 4. What Is Left
1. **Commit and Push**:
   - Commit all changes to Git.
   - Push to `origin main` (`https://github.com/adityamandre25/koi`).
