// ============================================================
//  editor/theme.rs
//  Paleta de cores e tema visual do RS2BR Engine Editor
//  Baseado no design "Atom One Dark" adaptado.
// ============================================================

use eframe::egui;

// ── Cores base ────────────────────────────────────────────
pub const BG_BASE: egui::Color32 = egui::Color32::from_rgb(26, 29, 33);      // #1a1d21
pub const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(33, 37, 43);     // #21252b
pub const BG_WIDGET: egui::Color32 = egui::Color32::from_rgb(40, 44, 52);    // #282c34
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(61, 68, 85);       // #3d4455
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(82, 148, 226);     // #5294e2
pub const ACCENT_FILL: egui::Color32 = egui::Color32::from_rgb(56, 109, 176);// darker accent for button bg
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(216, 222, 233); // #d8dee9
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(139, 149, 169); // #8b95a9
pub const GREEN: egui::Color32 = egui::Color32::from_rgb(92, 184, 92);       // #5cb85c
pub const YELLOW: egui::Color32 = egui::Color32::from_rgb(240, 192, 96);     // #f0c060
pub const RED: egui::Color32 = egui::Color32::from_rgb(224, 92, 92);         // #e05c5c

/// Aplica o tema visual ao contexto egui.
/// Deve ser chamado no início de cada frame no `update()`.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT_PRIMARY);

    // Fundos principais
    visuals.panel_fill = BG_PANEL;
    visuals.faint_bg_color = BG_WIDGET;
    visuals.extreme_bg_color = BG_BASE;
    visuals.code_bg_color = BG_WIDGET;
    visuals.window_fill = BG_PANEL;

    // Widgets normais (labels, separadores, etc.)
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = BG_WIDGET;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, BORDER);

    // Widgets inativos (botões não pressionados)
    visuals.widgets.inactive.bg_fill = BG_WIDGET;
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(46, 51, 61);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);

    // Widgets com hover
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(52, 58, 70);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(56, 63, 77);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);

    // Widgets ativos (pressionados)
    visuals.widgets.active.bg_fill = ACCENT_FILL;
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(56, 109, 176);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);

    // Widgets abertos (dropdowns, etc.)
    visuals.widgets.open.bg_fill = BG_WIDGET;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, BORDER);

    // Seleção
    visuals.selection.bg_fill = egui::Color32::from_rgb(56, 93, 148);
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);

    // Links / destaque
    visuals.hyperlink_color = ACCENT;

    // Janelas e menus
    visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.window_rounding = egui::Rounding::same(4.0);
    visuals.menu_rounding = egui::Rounding::same(4.0);

    // Arredondamentos consistentes
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(3.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(3.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(3.0);
    visuals.widgets.active.rounding = egui::Rounding::same(3.0);
    visuals.widgets.open.rounding = egui::Rounding::same(3.0);

    ctx.set_visuals(visuals);

    // Zona clicável do divisor entre painéis (↔ / ↕) — "column resize handle" mais fácil de pegar.
    ctx.style_mut(|style| {
        // Sem tween em CollapsingHeader / animate_bool — painéis não “crescem” frame a frame.
        style.animation_time = 0.0;
        // Área do “resize handle” entre painéis — mais fácil de apanhar sem comer o conteúdo.
        style.interaction.resize_grab_radius_side = 11.0;
        style.interaction.resize_grab_radius_corner = 12.0;
    });
}
