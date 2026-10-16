//! Stage-entry sequences (`#Application/*.seq.xml`).
//!
//! A `ChangeStage` unit names the stage archive and, per trial-mission type,
//! its score settings:
//!
//! ```xml
//! <SequenceUnit><type>ChangeStage</type><param>
//!   <ArchiveName>ActD_MykonosAct1</ArchiveName>
//!   <Switch><Case flag="eFlag_TrialMissionType" operation="E" value="eFlag_TrialMissionType_Default">
//!     <RankTable><S>150000</S>...<E>0</E></RankTable>
//!     <BaseTime>1200</BaseTime><CheckPointNum>1</CheckPointNum>
//!     <EnableScore>Time</EnableScore>...
//! </Case></Switch></param></SequenceUnit>
//! ```

use anyhow::{Context, Result};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StageScoreSettings {
    /// S, A, B, C, D, E thresholds when present.
    pub rank_table: Option<[u32; 6]>,
    pub base_time: Option<f32>,
    pub checkpoint_num: Option<u32>,
    /// Empty = every category counts.
    pub enable_score: Vec<String>,
}

/// Score settings for `archive` under trial-mission `mission`
/// (e.g. "eFlag_TrialMissionType_Default"), from one sequence file.
pub fn stage_score_settings(bytes: &[u8], archive: &str, mission: &str) -> Result<Option<StageScoreSettings>> {
    let text = std::str::from_utf8(bytes).context("sequence is not UTF-8")?.trim_start_matches('\u{feff}');
    let doc = roxmltree::Document::parse(text).context("parse sequence XML")?;
    let child_text = |n: roxmltree::Node, name: &str| {
        n.children().find(|c| c.has_tag_name(name)).and_then(|c| c.text()).map(str::trim).map(str::to_owned)
    };
    for unit in doc.descendants().filter(|n| n.has_tag_name("SequenceUnit")) {
        if child_text(unit, "type").as_deref() != Some("ChangeStage") {
            continue;
        }
        let Some(param) = unit.children().find(|c| c.has_tag_name("param")) else { continue };
        if child_text(param, "ArchiveName").as_deref() != Some(archive) {
            continue;
        }
        // Settings live in the matching <Case>, or directly in <param>.
        let scope = param
            .descendants()
            .find(|n| n.has_tag_name("Case") && n.attribute("value") == Some(mission))
            .unwrap_or(param);
        let mut out = StageScoreSettings::default();
        if let Some(table) = scope.children().find(|c| c.has_tag_name("RankTable")) {
            let mut t = [0u32; 6];
            for (i, r) in ["S", "A", "B", "C", "D", "E"].iter().enumerate() {
                t[i] = child_text(table, r).and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            out.rank_table = Some(t);
        }
        out.base_time = child_text(scope, "BaseTime").and_then(|v| v.parse().ok());
        out.checkpoint_num = child_text(scope, "CheckPointNum").and_then(|v| v.parse().ok());
        out.enable_score = scope
            .children()
            .filter(|c| c.has_tag_name("EnableScore"))
            .filter_map(|c| c.text().map(|t| t.trim().to_owned()))
            .collect();
        return Ok(Some(out));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_matching_case() {
        let xml = r#"<?xml version="1.0"?><MicroSequence><SequenceUnit><type>ChangeStage</type><param>
            <ArchiveName>ActD_MykonosAct1</ArchiveName>
            <Switch>
              <Case flag="f" operation="E" value="other"><BaseTime>500</BaseTime></Case>
              <Case flag="f" operation="E" value="eFlag_TrialMissionType_Default">
                <RankTable><S>150000</S><A>135000</A><B>100000</B><C>50000</C><D>25000</D><E>0</E></RankTable>
                <BaseTime>1200</BaseTime><CheckPointNum>1</CheckPointNum>
                <EnableScore>Time</EnableScore><EnableScore>Ring</EnableScore>
              </Case>
            </Switch></param></SequenceUnit></MicroSequence>"#;
        let s = stage_score_settings(xml.as_bytes(), "ActD_MykonosAct1", "eFlag_TrialMissionType_Default")
            .unwrap()
            .unwrap();
        assert_eq!(s.rank_table, Some([150000, 135000, 100000, 50000, 25000, 0]));
        assert_eq!(s.base_time, Some(1200.0));
        assert_eq!(s.checkpoint_num, Some(1));
        assert_eq!(s.enable_score, vec!["Time", "Ring"]);
    }
}
