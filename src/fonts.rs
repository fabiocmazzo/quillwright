//! Startup-only font discovery and resolution; no changes to document text.
use egui::{FontData, FontDefinitions, FontFamily};
use fontdb::{Database, Family, Query, Style, Weight};

const STYLES: [&str; 4] = ["regular", "bold", "italic", "bold-italic"];
pub(crate) const LITERATA: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Literata-Regular.ttf"),
    include_bytes!("../assets/fonts/Literata-Bold.ttf"),
    include_bytes!("../assets/fonts/Literata-Italic.ttf"),
    include_bytes!("../assets/fonts/Literata-BoldItalic.ttf"),
];
const ATKINSON: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/AtkinsonHyperlegible-Regular.ttf"),
    include_bytes!("../assets/fonts/AtkinsonHyperlegible-Bold.ttf"),
    include_bytes!("../assets/fonts/AtkinsonHyperlegible-Italic.ttf"),
    include_bytes!("../assets/fonts/AtkinsonHyperlegible-BoldItalic.ttf"),
];

#[derive(Clone, Copy)]
pub struct Typography {
    pub size: f32,
    pub line_height: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            size: 18.0,
            line_height: 1.4,
        }
    }
}

pub fn typography(ctx: &egui::Context) -> Typography {
    ctx.data(|data| data.get_temp(egui::Id::unique("editor_typography")))
        .unwrap_or_default()
}

pub fn family(bold: bool, italic: bool) -> FontFamily {
    FontFamily::Name(format!("editor-{}", STYLES[bold as usize + 2 * italic as usize]).into())
}

fn bundled(name: &str) -> Option<[&'static [u8]; 4]> {
    if name.eq_ignore_ascii_case("Literata") {
        Some(LITERATA)
    } else if name.eq_ignore_ascii_case("Atkinson Hyperlegible") {
        Some(ATKINSON)
    } else {
        None
    }
}

fn system_face(db: &Database, name: &str, variant: usize) -> Option<FontData> {
    let canonical = db
        .faces()
        .flat_map(|face| &face.families)
        .find(|(family, _)| family.eq_ignore_ascii_case(name))?
        .0
        .as_str();
    let weight = if variant % 2 == 1 {
        Weight::BOLD
    } else {
        Weight::NORMAL
    };
    let style = if variant >= 2 {
        Style::Italic
    } else {
        Style::Normal
    };
    let id = db.query(&Query {
        families: &[Family::Name(canonical)],
        weight,
        style,
        ..Default::default()
    })?;
    let face = db.face(id)?;
    // Do not silently use an upright regular face for a missing bold/italic face.
    // Use a real bundled variant instead. Variable axes are not exposed by egui.
    if face.weight != weight
        || !(face.style == style || (style == Style::Italic && face.style == Style::Oblique))
    {
        return None;
    }
    db.with_face_data(id, |bytes, index| {
        ab_glyph::FontRef::try_from_slice_and_index(bytes, index).ok()?;
        let mut data = FontData::from_owned(bytes.to_vec());
        data.index = index; // Preserve the selected face in TTC/OTC collections.
        Some(data)
    })?
}

fn definitions(db: &Database, name: &str) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let egui_fallback = fonts.families[&FontFamily::Proportional].clone();
    for (family_name, bytes) in [("Literata", LITERATA), ("Atkinson Hyperlegible", ATKINSON)] {
        for (style, data) in STYLES.iter().zip(bytes) {
            fonts.font_data.insert(
                format!("{family_name}-{style}"),
                std::sync::Arc::new(FontData::from_static(data)),
            );
        }
    }
    for (variant, style) in STYLES.iter().enumerate() {
        let mut chain = Vec::new();
        if let Some(data) = system_face(db, name.trim(), variant) {
            let key = format!("system-{}", style);
            fonts
                .font_data
                .insert(key.clone(), std::sync::Arc::new(data));
            chain.push(key);
        }
        if bundled(name.trim()).is_some() {
            let canonical = if name.trim().eq_ignore_ascii_case("Literata") {
                "Literata"
            } else {
                "Atkinson Hyperlegible"
            };
            chain.push(format!("{canonical}-{}", style));
        }
        let default_key = format!("Literata-{}", style);
        if !chain.contains(&default_key) {
            chain.push(default_key);
        }
        chain.extend(egui_fallback.clone());
        fonts
            .families
            .insert(family(variant % 2 == 1, variant >= 2), chain);
    }
    fonts
}

pub fn install(ctx: &egui::Context, config: &crate::editor::EditorConfig) {
    let mut db = Database::new();
    db.load_system_fonts();
    install_with_database(ctx, config, &db);
}

fn install_with_database(ctx: &egui::Context, config: &crate::editor::EditorConfig, db: &Database) {
    let defaults = Typography::default();
    let finite_positive = |value: f32, fallback| {
        if value.is_finite() && value > 0.0 {
            value
        } else {
            fallback
        }
    };
    let metrics = Typography {
        size: finite_positive(config.font_size, defaults.size).clamp(6.0, 96.0),
        line_height: finite_positive(config.line_height, defaults.line_height).clamp(1.0, 3.0),
    };
    ctx.set_fonts(definitions(db, &config.font_family));
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique("editor_typography"), metrics));
}

/// Change the editor text size at runtime (fonts stay installed). Returns the clamped size.
pub fn set_size(ctx: &egui::Context, size: f32) -> f32 {
    let mut metrics = typography(ctx);
    metrics.size = if size.is_finite() {
        size.clamp(6.0, 96.0)
    } else {
        Typography::default().size
    };
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique("editor_typography"), metrics));
    metrics.size
}

#[cfg(test)]
pub fn install_test_fonts(ctx: &egui::Context) {
    install_with_database(
        ctx,
        &crate::editor::EditorConfig::default(),
        &Database::new(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fallback_and_all_real_variants_are_valid() {
        let db = Database::new();
        for (name, expected) in [
            ("Literata", "Literata"),
            ("atkinson hyperlegible", "Atkinson Hyperlegible"),
            ("missing family", "Literata"),
        ] {
            let fonts = definitions(&db, name);
            for (variant, style) in STYLES.iter().enumerate() {
                let chain = &fonts.families[&family(variant % 2 == 1, variant >= 2)];
                assert_eq!(chain[0], format!("{expected}-{}", style));
                let data = &fonts.font_data[&chain[0]];
                assert!(ab_glyph::FontRef::try_from_slice_and_index(
                    (*data.font).as_ref(),
                    data.index
                )
                .is_ok());
            }
        }
    }

    #[test]
    fn installed_family_wins_and_missing_styles_use_real_bundled_faces() {
        let mut db = Database::new();
        db.load_font_data(ATKINSON[0].to_vec());
        let name = db.faces().next().unwrap().families[0].0.clone();
        let fonts = definitions(&db, &name);
        assert_eq!(fonts.families[&family(false, false)][0], "system-regular");
        assert!(!fonts.families[&family(true, true)][0].starts_with("system"));
        for bytes in &ATKINSON[1..] {
            db.load_font_data(bytes.to_vec());
        }
        let fonts = definitions(&db, &name);
        for (variant, style) in STYLES.iter().enumerate() {
            assert_eq!(
                fonts.families[&family(variant % 2 == 1, variant >= 2)][0],
                format!("system-{}", style)
            );
        }
    }

    #[test]
    fn invalid_metrics_are_sanitized() {
        let ctx = egui::Context::default();
        let config = crate::editor::EditorConfig {
            font_size: f32::NAN,
            line_height: -1.0,
            ..Default::default()
        };
        install_with_database(&ctx, &config, &Database::new());
        assert_eq!(typography(&ctx).size, 18.0);
        assert_eq!(typography(&ctx).line_height, 1.4);
    }
    #[test]
    fn font_changes_keep_source_mapping_hit_testing_and_real_styles() {
        let source = "# Heading\n\nA **bold** and *italic* plus ***both*** paragraph with café and Unicode Ω. More words to wrap.";
        for name in ["Literata", "Atkinson Hyperlegible", "not-installed"] {
            for size in [14.0, 24.0] {
                let ctx = egui::Context::default();
                let config = crate::editor::EditorConfig {
                    font_family: name.into(),
                    font_size: size,
                    line_height: 1.6,
                    ..Default::default()
                };
                install_with_database(&ctx, &config, &Database::new());
                ctx.run_ui(Default::default(), |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        for active in [None, Some((0, source.chars().count()))] {
                            let projection =
                                crate::live_preview::project(ui, source, 180.0, active);
                            let display = projection.job.text
                                [..projection.job.text.find("café").unwrap()]
                                .chars()
                                .count();
                            let src = source[..source.find("café").unwrap()].chars().count();
                            assert_eq!(projection.to_source(display), src);
                            assert_eq!(projection.to_display(src), display);
                            if active.is_none() {
                                for (word, bold, italic) in [
                                    ("bold", true, false),
                                    ("italic", false, true),
                                    ("both", true, true),
                                ] {
                                    let index = projection.job.text.find(word).unwrap();
                                    let section = projection
                                        .job
                                        .sections
                                        .iter()
                                        .find(|s| s.byte_range.contains(&index.into()))
                                        .unwrap();
                                    assert_eq!(section.format.font_id.family, family(bold, italic));
                                    assert!(!section.format.italics); // real italic face, no synthetic slant
                                }
                            }
                            let galley = ui.painter().layout_job(projection.job.clone());
                            let mut cursor = egui::text::CCursor::new(display);
                            cursor.prefer_next_row = true;
                            let rect = galley.pos_from_cursor(cursor);
                            let hit = galley
                                .cursor_from_pos(egui::vec2(rect.left() + 0.1, rect.center().y));
                            assert_eq!(projection.to_source(hit.index.0), src, "{name} at {size}");
                            assert!(galley.rows.len() > 3);
                        }
                        let empty = crate::live_preview::project(ui, "", 180.0, Some((0, 0)));
                        let galley = ui.painter().layout_job(empty.job);
                        assert!((galley.rows[0].rect().height() - size * 1.6).abs() < 2.0);
                    });
                })
                .drop_without_applying_deltas();
            }
        }
    }
}
