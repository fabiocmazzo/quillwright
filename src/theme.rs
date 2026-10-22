//! Writing-oriented color themes. The editor page is lighter (or darker) than the
//! surrounding "desk" so the text column reads like a sheet of paper.
use egui::{epaint::Shadow, Color32, Context, CornerRadius, Stroke, Style, Visuals};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Paper,
    Sepia,
    Night,
}

impl Preset {
    pub const ALL: [Preset; 3] = [Preset::Paper, Preset::Sepia, Preset::Night];

    /// Accepts the preset names plus the legacy md-echo values "light" and "dark".
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_lowercase().as_str() {
            "paper" | "light" => Preset::Paper,
            "sepia" => Preset::Sepia,
            _ => Preset::Night,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Preset::Paper => "paper",
            Preset::Sepia => "sepia",
            Preset::Night => "night",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Preset::Paper => "Paper",
            Preset::Sepia => "Sepia",
            Preset::Night => "Night",
        }
    }

    pub fn palette(self) -> Palette {
        let hex = |v: u32| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
        match self {
            Preset::Paper => Palette {
                dark: false,
                desk: hex(0xEDE8DF),
                page: hex(0xFBF9F4),
                text: hex(0x2B2722),
                muted: hex(0x857C72),
                accent: hex(0xA4532F),
            },
            Preset::Sepia => Palette {
                dark: false,
                desk: hex(0xE4D6BC),
                page: hex(0xF4EBD7),
                text: hex(0x3F3122),
                muted: hex(0x86735C),
                accent: hex(0x8C5A2B),
            },
            Preset::Night => Palette {
                dark: true,
                desk: hex(0x16171B),
                page: hex(0x1F2026),
                text: hex(0xDCD7CE),
                muted: hex(0x8C877E),
                accent: hex(0xD9A066),
            },
        }
    }
}

/// The theme setting that follows the system: Paper, or Night when the system is in
/// dark mode. It is the default.
pub const SYSTEM: &str = "system";

/// True when the theme setting follows the system's light or dark mode.
pub fn follows_system(name: &str) -> bool {
    let name = name.trim();
    name.is_empty() || name.eq_ignore_ascii_case(SYSTEM)
}

/// The preset for a theme setting: a preset name, or [`SYSTEM`] with the system's theme
/// (unknown counts as light).
pub fn choose(name: &str, system: Option<egui::Theme>) -> Preset {
    if !follows_system(name) {
        return Preset::from_name(name);
    }
    match system {
        Some(egui::Theme::Dark) => Preset::Night,
        _ => Preset::Paper,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    /// Panels, menus, and sidebar.
    pub desk: Color32,
    /// The editor canvas.
    pub page: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
}

/// Linear blend from `a` (t = 0) to `b` (t = 1).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

fn visuals(p: &Palette) -> Visuals {
    let mut v = if p.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let border = mix(p.desk, p.text, 0.14);
    let raised = mix(p.desk, p.page, 0.6);

    v.override_text_color = Some(p.text);
    v.panel_fill = p.desk;
    v.window_fill = raised;
    v.extreme_bg_color = p.page;
    v.faint_bg_color = mix(p.desk, p.text, 0.04);
    v.code_bg_color = mix(p.page, p.text, 0.07);
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.accent;
    v.selection.bg_fill = mix(p.page, p.accent, if p.dark { 0.35 } else { 0.25 });
    v.selection.stroke = Stroke::new(1.0_f32, p.text);
    v.text_cursor.stroke = Stroke::new(2.0_f32, p.accent);
    v.window_stroke = Stroke::new(1.0_f32, border);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    let shadow_alpha = if p.dark { 110 } else { 40 };
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha / 2),
    };
    v.collapsing_header_frame = false;
    v.indent_has_left_vline = false;
    v.striped = false;

    let rounding = CornerRadius::same(6);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.desk;
    w.noninteractive.weak_bg_fill = p.desk;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, border);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.noninteractive.corner_radius = rounding;
    for (state, amount) in [
        (&mut w.inactive, 0.07),
        (&mut w.hovered, 0.13),
        (&mut w.active, 0.2),
        (&mut w.open, 0.13),
    ] {
        state.bg_fill = mix(p.desk, p.text, amount);
        state.weak_bg_fill = mix(p.desk, p.text, amount);
        state.bg_stroke = Stroke::NONE;
        state.fg_stroke = Stroke::new(1.0_f32, p.text);
        state.corner_radius = rounding;
        state.expansion = 0.0;
    }
    w.inactive.weak_bg_fill = mix(p.desk, p.text, 0.05);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, mix(p.desk, p.accent, 0.5));
    v
}

pub struct Overrides {
    pub background: Option<Color32>,
    pub panel: Option<Color32>,
    pub text: Option<Color32>,
    pub accent: Option<Color32>,
}

/// Build the palette for a preset with the user's color overrides applied.
pub fn resolve(preset: Preset, overrides: &Overrides) -> Palette {
    let mut p = preset.palette();
    if let Some(c) = overrides.background {
        p.page = c;
    }
    if let Some(c) = overrides.panel {
        p.desk = c;
    }
    if let Some(c) = overrides.text {
        p.text = c;
        p.muted = mix(c, p.page, 0.45);
    }
    if let Some(c) = overrides.accent {
        p.accent = c;
    }
    p
}

pub fn apply(ctx: &Context, palette: &Palette, hyperlink: Option<Color32>) {
    let mut visuals = visuals(palette);
    if let Some(color) = hyperlink {
        visuals.hyperlink_color = color;
    }
    let mut style: Style = (*ctx.global_style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.spacing.menu_margin = egui::Margin::same(8);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.indent = 14.0;
    style.spacing.interact_size.y = 24.0;
    // egui keeps a light and a dark style and follows the system theme; Quillwright's
    // palette is chosen in its own settings, so both get the same style.
    ctx.all_styles_mut(|s| *s = style.clone());
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("writing_palette"), *palette));
}

/// The palette installed by [`apply`], for widgets that need muted or accent colors.
pub fn current(ctx: &Context) -> Palette {
    ctx.data(|d| d.get_temp(egui::Id::unique("writing_palette")))
        .unwrap_or_else(|| Preset::Night.palette())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_names_map_to_presets() {
        assert_eq!(Preset::from_name("dark"), Preset::Night);
        assert_eq!(Preset::from_name("Light"), Preset::Paper);
        assert_eq!(Preset::from_name("sepia"), Preset::Sepia);
        for preset in Preset::ALL {
            assert_eq!(Preset::from_name(preset.name()), preset);
        }
    }

    #[test]
    fn system_setting_follows_light_and_dark_mode() {
        assert_eq!(choose("system", Some(egui::Theme::Dark)), Preset::Night);
        assert_eq!(choose("System", Some(egui::Theme::Light)), Preset::Paper);
        assert_eq!(choose("", None), Preset::Paper);
        // A chosen preset wins over the system.
        assert_eq!(choose("night", Some(egui::Theme::Light)), Preset::Night);
        assert_eq!(choose("sepia", Some(egui::Theme::Dark)), Preset::Sepia);
    }

    #[test]
    fn mix_blends_endpoints() {
        let a = Color32::from_rgb(0, 100, 200);
        let b = Color32::from_rgb(200, 100, 0);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(a, b, 0.5), Color32::from_rgb(100, 100, 100));
    }
}
