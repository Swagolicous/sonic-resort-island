//! Stage score and rank, ported from the score manager in default.xex
//! (sub_8245EDE0 .. sub_8245F200, results assembly sub_8235C908).
//!
//! Global tuning comes from the executable's .data (the values the debug
//! editor exposes, registered by sub_8245F2A8); per-stage settings (rank
//! table, base time, checkpoint count, enabled categories) come from the
//! stage's ChangeStage sequence entry in `#Application/*.seq.xml`.

/// Score categories in result order (sub_8235C908 sums the enabled ones).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreKind {
    Time,
    Ring,
    Speed,
    Enemy,
    Trick,
    Combo,
    Crush,
}

impl ScoreKind {
    pub const ALL: [ScoreKind; 7] = [
        ScoreKind::Time,
        ScoreKind::Ring,
        ScoreKind::Speed,
        ScoreKind::Enemy,
        ScoreKind::Trick,
        ScoreKind::Combo,
        ScoreKind::Crush,
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| format!("{k:?}") == name)
    }
}

/// Executable defaults at 0x83284300.. (names from the editor registration).
#[derive(Clone, Debug)]
pub struct ScoreParams {
    pub time_bonus_efficient: u32,
    pub ring_bonus_efficient: u32,
    pub speed_bonus_side_view_scale: f32,
    pub speed_bonus_normal_scale: f32,
    pub speed_bonus_boost_scale: f32,
    pub speed_bonus_bias: u32,
    pub speed_bonus_efficient: u32,
    pub enemy_combo: u32,
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self {
            time_bonus_efficient: 100,
            ring_bonus_efficient: 100,
            speed_bonus_side_view_scale: 2.0,
            speed_bonus_normal_scale: 0.8,
            speed_bonus_boost_scale: 1.0,
            speed_bonus_bias: 20,
            speed_bonus_efficient: 220,
            enemy_combo: 300,
        }
    }
}

/// Per-stage settings. Defaults are the stage-config constructor's
/// (BaseTime 0, CheckPointNum 3, everything enabled) with the score manager's
/// default rank table (sub_8245F200).
#[derive(Clone, Debug)]
pub struct StageScore {
    /// Minimum totals for S, A, B, C, D, E.
    pub rank_table: [u32; 6],
    pub base_time: f32,
    pub checkpoint_num: u32,
    pub enabled: Vec<ScoreKind>,
}

impl Default for StageScore {
    fn default() -> Self {
        Self {
            rank_table: [150_000, 100_000, 75_000, 50_000, 25_000, 0],
            base_time: 0.0,
            checkpoint_num: 3,
            enabled: ScoreKind::ALL.to_vec(),
        }
    }
}

/// What the score manager accumulates during a run.
#[derive(Clone, Debug, Default)]
pub struct ScoreTracker {
    /// Best speed samples taken at checkpoints (already scaled).
    samples: Vec<f32>,
    /// Fallback sample when the stage counts no checkpoints (sub_8245EEC0).
    last_speed: f32,
    pub enemy: u32,
    pub trick: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScoreResults {
    pub time_bonus: u32,
    pub ring_bonus: u32,
    pub speed_bonus: u32,
    pub enemy_bonus: u32,
    pub trick_bonus: u32,
    pub total: u32,
    pub rank: char,
}

fn scaled_speed(p: &ScoreParams, speed: f32, boosting: bool, side_view: bool) -> f32 {
    let mut v = speed * if boosting { p.speed_bonus_boost_scale } else { p.speed_bonus_normal_scale };
    if side_view {
        v *= p.speed_bonus_side_view_scale;
    }
    v
}

impl ScoreTracker {
    /// Speed sample at a checkpoint; keeps the best `checkpoint_num` (sub_8245EDF0).
    pub fn record_checkpoint_speed(&mut self, p: &ScoreParams, stage: &StageScore, speed: f32, boosting: bool, side_view: bool) {
        let v = scaled_speed(p, speed, boosting, side_view);
        let n = stage.checkpoint_num.min(3) as usize;
        if n == 0 {
            return;
        }
        if self.samples.len() < n {
            self.samples.push(v);
        } else if let Some(min) = self.samples.iter_mut().min_by(|a, b| a.total_cmp(b)) {
            if *min < v {
                *min = v;
            }
        }
    }

    /// Speed used when the stage has no checkpoint samples (sub_8245EEC0).
    pub fn set_last_speed(&mut self, p: &ScoreParams, speed: f32, boosting: bool, side_view: bool) {
        self.last_speed = scaled_speed(p, speed, boosting, side_view);
    }

    /// Enemy defeated (sub_8245EEF8): its score, plus the combo bonus.
    pub fn add_enemy(&mut self, p: &ScoreParams, score: u32, combo: bool) {
        self.enemy += score + if combo { p.enemy_combo } else { 0 };
    }

    /// sub_8245F0F0: (mean of the samples - bias) * efficient, floored at 0.
    pub fn speed_bonus(&self, p: &ScoreParams, stage: &StageScore) -> u32 {
        let n = stage.checkpoint_num.min(3) as usize;
        let mean = if n == 0 {
            self.last_speed
        } else {
            // Unfilled slots count as zero, as in the game's fixed array.
            self.samples.iter().sum::<f32>() / n as f32
        };
        ((mean - p.speed_bonus_bias as f32).max(0.0) * p.speed_bonus_efficient as f32) as u32
    }

    /// The results screen's numbers (sub_8235C908).
    pub fn results(&self, p: &ScoreParams, stage: &StageScore, time: f32, rings: u32) -> ScoreResults {
        // sub_8245F0B0: (BaseTime - time) * TimeBonusEfficient, floored at 0.
        let time_bonus = ((stage.base_time - time).max(0.0) * p.time_bonus_efficient as f32) as u32;
        // sub_8245EDE0
        let ring_bonus = p.ring_bonus_efficient * rings;
        let speed_bonus = self.speed_bonus(p, stage);
        let parts = [
            (ScoreKind::Time, time_bonus),
            (ScoreKind::Ring, ring_bonus),
            (ScoreKind::Speed, speed_bonus),
            (ScoreKind::Enemy, self.enemy),
            (ScoreKind::Trick, self.trick),
        ];
        let total = parts.iter().filter(|(k, _)| stage.enabled.contains(k)).map(|(_, v)| v).sum();
        ScoreResults {
            time_bonus,
            ring_bonus,
            speed_bonus,
            enemy_bonus: self.enemy,
            trick_bonus: self.trick,
            total,
            rank: rank(stage, total),
        }
    }
}

/// First rank whose threshold the total reaches (sub_8245F010).
pub fn rank(stage: &StageScore, total: u32) -> char {
    let i = stage.rank_table.iter().position(|t| total >= *t).unwrap_or(5);
    ['S', 'A', 'B', 'C', 'D', 'E'][i]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windmill_isle_act1() -> StageScore {
        // SR_EnterMykonosDayTutorial.seq.xml, eFlag_TrialMissionType_Default.
        StageScore {
            rank_table: [150_000, 135_000, 100_000, 50_000, 25_000, 0],
            base_time: 1200.0,
            checkpoint_num: 1,
            enabled: vec![ScoreKind::Time, ScoreKind::Ring, ScoreKind::Speed, ScoreKind::Enemy],
        }
    }

    #[test]
    fn scores_a_run() {
        let p = ScoreParams::default();
        let stage = windmill_isle_act1();
        let mut t = ScoreTracker::default();
        t.record_checkpoint_speed(&p, &stage, 50.0, false, false); // 40 scaled
        t.record_checkpoint_speed(&p, &stage, 80.0, true, false); // 80 scaled: best of 1
        t.add_enemy(&p, 100, false);
        let r = t.results(&p, &stage, 60.0, 72);
        assert_eq!(r.time_bonus, 114_000);
        assert_eq!(r.ring_bonus, 7_200);
        assert_eq!(r.speed_bonus, (80 - 20) * 220);
        assert_eq!(r.total, 114_000 + 7_200 + 13_200 + 100);
        // 134_500: just short of A (135_000).
        assert_eq!(r.rank, 'B');
    }

    #[test]
    fn rank_thresholds() {
        let stage = windmill_isle_act1();
        assert_eq!(rank(&stage, 150_000), 'S');
        assert_eq!(rank(&stage, 149_999), 'A');
        assert_eq!(rank(&stage, 24_999), 'E');
    }
}
