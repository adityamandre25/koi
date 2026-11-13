use std::f64::consts::TAU;

use git_koi::behavior::Behavior;
use git_koi::motion::{Course, SwimPose, calculate_swim_pose, frame_count};

type PoseValue = (&'static str, fn(&SwimPose) -> f64, bool);

const POSE_VALUES: [PoseValue; 15] = [
    ("lap angle", read_lap_angle, true),
    ("wave phase", |pose| pose.wave_phase_radians, true),
    ("wave strength", |pose| pose.wave_strength, false),
    ("left tail lobe", |pose| pose.tail_lobe_bends_radians[0], false),
    ("right tail lobe", |pose| pose.tail_lobe_bends_radians[1], false),
    ("tail spread", |pose| pose.tail_spread_radians, false),
    ("left side fin", |pose| pose.side_fin_openings_radians[0], false),
    ("right side fin", |pose| pose.side_fin_openings_radians[1], false),
    ("left side fin bend", |pose| pose.side_fin_bends_radians[0], false),
    ("right side fin bend", |pose| pose.side_fin_bends_radians[1], false),
    ("pelvic fins", |pose| pose.pelvic_fin_opening_radians, false),
    ("whiskers", |pose| pose.whisker_sway_radians, false),
    ("depth", |pose| pose.depth, false),
    ("mouth opening", |pose| pose.mouth_opening, false),
    ("side fin flare", |pose| pose.side_fin_openings_radians[1] - pose.side_fin_openings_radians[0], false),
];

#[test]
fn all_swimming_behaviors_have_seamless_loops() {
    for behavior in [
        Behavior::PeacefulGliding,
        Behavior::GentleSwimming,
        Behavior::EnergeticSwimming,
        Behavior::FastSwimming,
    ] {
        let count = frame_count(behavior);
        assert_eq!(count, 120);

        let pose_start = calculate_swim_pose(behavior, 0);
        let pose_end_wrap = calculate_swim_pose(behavior, count);
        assert_eq!(pose_start, pose_end_wrap, "{behavior:?} wraps seamlessly");
    }
}

#[test]
fn every_pose_value_flows_across_the_loop_boundary() {
    for behavior in [
        Behavior::PeacefulGliding,
        Behavior::GentleSwimming,
        Behavior::EnergeticSwimming,
        Behavior::FastSwimming,
    ] {
        let count = frame_count(behavior);
        let poses: Vec<SwimPose> = (0..=count).map(|i| calculate_swim_pose(behavior, i)).collect();

        for (name, read_val, is_angle) in POSE_VALUES {
            for window in poses.windows(2) {
                let p0 = read_val(&window[0]);
                let p1 = read_val(&window[1]);
                let diff = if is_angle {
                    ((p1 - p0 + TAU / 2.0).rem_euclid(TAU) - TAU / 2.0).abs()
                } else {
                    (p1 - p0).abs()
                };
                assert!(diff < 1.5, "{behavior:?} {name} jump too large: {diff}");
            }
        }
    }
}

#[test]
fn fast_swimming_has_stronger_wave_strength_than_gliding() {
    let glide_poses = collect_loop(Behavior::PeacefulGliding);
    let fast_poses = collect_loop(Behavior::FastSwimming);

    let max_glide_wave: f64 = glide_poses.iter().map(|p| p.wave_strength).fold(0.0, f64::max);
    let max_fast_wave: f64 = fast_poses.iter().map(|p| p.wave_strength).fold(0.0, f64::max);

    assert!(
        max_fast_wave > max_glide_wave,
        "fast swimming tail wave {max_fast_wave} should exceed glide {max_glide_wave}"
    );
}

#[test]
fn tail_lobes_trail_out_of_step() {
    for behavior in [
        Behavior::PeacefulGliding,
        Behavior::GentleSwimming,
        Behavior::EnergeticSwimming,
        Behavior::FastSwimming,
    ] {
        let poses = collect_loop(behavior);
        let splits: Vec<f64> = poses
            .iter()
            .map(|p| (p.tail_lobe_bends_radians[1] - p.tail_lobe_bends_radians[0]).abs())
            .collect();
        let max_split = splits.into_iter().fold(0.0, f64::max);
        assert!(max_split > 0.05, "{behavior:?} tail lobes must split");
    }
}

#[test]
fn the_mouth_breathes_while_swimming() {
    for behavior in [
        Behavior::PeacefulGliding,
        Behavior::GentleSwimming,
        Behavior::EnergeticSwimming,
        Behavior::FastSwimming,
    ] {
        let poses = collect_loop(behavior);
        let max_opening = poses.iter().map(|p| p.mouth_opening).fold(0.0, f64::max);
        assert!(max_opening >= 0.5, "{behavior:?} mouth should breathe");
    }
}

fn collect_loop(behavior: Behavior) -> Vec<SwimPose> {
    (0..frame_count(behavior))
        .map(|i| calculate_swim_pose(behavior, i))
        .collect()
}

fn read_lap_angle(pose: &SwimPose) -> f64 {
    match pose.course {
        Course::Lap { angle_radians } => angle_radians,
        Course::Hover { .. } => 0.0,
    }
}
