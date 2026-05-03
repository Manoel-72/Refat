use macroquad::color::Color;
use macroquad::shapes::draw_rectangle;
use macroquad::window::{screen_height, screen_width};

/// Efeitos de tela cheia (flash por baixo, fade por cima).
#[derive(Debug, Clone)]
pub struct ScreenFx {
    fade_alpha: f32,
    fade_target: f32,
    fade_speed: f32,
    fade_r: f32,
    fade_g: f32,
    fade_b: f32,

    flash_alpha: f32,
    flash_decay: f32,
    flash_r: f32,
    flash_g: f32,
    flash_b: f32,
}

impl Default for ScreenFx {
    fn default() -> Self {
        Self {
            fade_alpha: 0.0,
            fade_target: 0.0,
            fade_speed: 0.0,
            fade_r: 0.0,
            fade_g: 0.0,
            fade_b: 0.0,
            flash_alpha: 0.0,
            flash_decay: 0.0,
            flash_r: 1.0,
            flash_g: 1.0,
            flash_b: 1.0,
        }
    }
}

impl ScreenFx {
    /// Overlay de alpha 1.0 → 0.0 (revela a cena).
    pub fn fade_in(&mut self, duration: f32) {
        let d = duration.max(1e-6);
        self.fade_alpha = 1.0;
        self.fade_target = 0.0;
        self.fade_speed = 1.0 / d;
    }

    /// Overlay de alpha 0.0 → 1.0 (escurece a cena).
    pub fn fade_out(&mut self, duration: f32) {
        let d = duration.max(1e-6);
        self.fade_alpha = 0.0;
        self.fade_target = 1.0;
        self.fade_speed = 1.0 / d;
    }

    pub fn flash(&mut self, r: f32, g: f32, b: f32, duration: f32) {
        let d = duration.max(1e-6);
        self.flash_r = r.clamp(0.0, 1.0);
        self.flash_g = g.clamp(0.0, 1.0);
        self.flash_b = b.clamp(0.0, 1.0);
        self.flash_alpha = 1.0;
        self.flash_decay = 1.0 / d;
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.max(0.0);

        if self.fade_speed > 0.0 {
            let step = self.fade_speed * dt;
            let d = self.fade_target - self.fade_alpha;
            if d.abs() <= step || step < 1e-9 {
                self.fade_alpha = self.fade_target;
                if (self.fade_alpha - self.fade_target).abs() < f32::EPSILON {
                    self.fade_speed = 0.0;
                }
            } else {
                self.fade_alpha += d.signum() * step;
            }
        }

        if self.flash_alpha > 0.001 {
            self.flash_alpha = (self.flash_alpha - self.flash_decay * dt).max(0.0);
        }
    }

    /// Desenha com macroquad em tela cheia (`screen_width` / `screen_height`).
    /// Flash primeiro, depois fade. Só desenha se alpha > 0.001.
    pub fn draw(&self) {
        let w = screen_width();
        let h = screen_height();
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        if self.flash_alpha > 0.001 {
            let c = Color::from_rgba(
                (self.flash_r * 255.0) as u8,
                (self.flash_g * 255.0) as u8,
                (self.flash_b * 255.0) as u8,
                (self.flash_alpha * 255.0).clamp(0.0, 255.0) as u8,
            );
            draw_rectangle(0.0, 0.0, w, h, c);
        }

        if self.fade_alpha > 0.001 {
            let c = Color::from_rgba(
                (self.fade_r * 255.0) as u8,
                (self.fade_g * 255.0) as u8,
                (self.fade_b * 255.0) as u8,
                (self.fade_alpha * 255.0).clamp(0.0, 255.0) as u8,
            );
            draw_rectangle(0.0, 0.0, w, h, c);
        }
    }

    /// Host egui (editor / janela eframe): mesmo overlay no retângulo da vista, após a cena.
    pub fn paint_egui(&self, painter: &egui::Painter, rect: egui::Rect) {
        if self.flash_alpha > 0.001 {
            painter.rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(
                    (self.flash_r * 255.0).clamp(0.0, 255.0) as u8,
                    (self.flash_g * 255.0).clamp(0.0, 255.0) as u8,
                    (self.flash_b * 255.0).clamp(0.0, 255.0) as u8,
                    (self.flash_alpha * 255.0).clamp(0.0, 255.0) as u8,
                ),
            );
        }
        if self.fade_alpha > 0.001 {
            painter.rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(
                    (self.fade_r * 255.0).clamp(0.0, 255.0) as u8,
                    (self.fade_g * 255.0).clamp(0.0, 255.0) as u8,
                    (self.fade_b * 255.0).clamp(0.0, 255.0) as u8,
                    (self.fade_alpha * 255.0).clamp(0.0, 255.0) as u8,
                ),
            );
        }
    }
}
