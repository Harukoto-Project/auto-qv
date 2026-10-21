use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigFile {
    pub sensitivity: SensitivitySection,
    pub pen: PenSection,
    pub layout: LayoutSection,
    pub log: LogSection,
    pub safety: SafetySection,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SensitivitySection {
    pub counts_per_em_x: Option<f32>,
    pub counts_per_em_y: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PenSection {
    pub button: Option<String>,
    pub step_delay_ms: Option<u32>,
    pub max_counts_per_step: Option<u32>,
    pub move_speed_multiplier: Option<f32>,
    pub min_pen_down_interval_ms: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutSection {
    pub size: Option<f32>,
    pub char_spacing: Option<f32>,
    pub line_spacing: Option<f32>,
    pub line_width_em: Option<f32>,
    pub kinsoku: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogSection {
    pub missing_glyph_log: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SafetySection {
    pub start_hotkey: Option<String>,
    pub abort_hotkey: Option<String>,
}
