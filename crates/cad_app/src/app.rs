// Screen-space drawing uses `f32` (egui's pixel type) for values sourced
// from `f64` world coordinates; see camera.rs for why the truncation is fine.
#![allow(clippy::cast_possible_truncation)]

use cad_core::Project;
use cad_render::RenderPrimitive;

use crate::camera::Camera;
use crate::demo::seed_demo_project;

/// The skeleton `04_UI_와이어프레임_정책.md` layout: menu, tool bar, left
/// (layers), center (canvas), right (validation), bottom (status). Tool
/// buttons and file menu items are placeholders wired up in later Phase 8
/// steps — this pass only proves the render pipeline and pan/zoom work.
pub struct CadApp {
    project: Project,
    camera: Camera,
}

impl Default for CadApp {
    fn default() -> Self {
        Self {
            project: seed_demo_project(),
            camera: Camera::default(),
        }
    }
}

impl eframe::App for CadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.menu_bar(ctx);
        Self::tool_bar(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        let status = self.canvas(ctx);
        self.status_bar(ctx, &status);
    }
}

impl CadApp {
    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("파일", |ui| {
                    ui.weak("새 프로젝트 (Phase 8d 예정)");
                    ui.weak("열기 (Phase 8d 예정)");
                    ui.weak("저장 (Phase 8d 예정)");
                });
                ui.menu_button("편집", |ui| {
                    ui.weak("실행 취소 (Phase 8b 예정)");
                    ui.weak("다시 실행 (Phase 8b 예정)");
                });
                ui.menu_button("보기", |ui| {
                    if ui.button("보기 초기화").clicked() {
                        self.camera = Camera::default();
                    }
                });
            });
        });
    }

    fn tool_bar(ctx: &egui::Context) {
        egui::TopBottomPanel::top("tool_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for label in ["선택", "선", "사각형", "원", "호", "텍스트", "치수"] {
                    ui.add_enabled(false, egui::Button::new(label));
                }
                ui.separator();
                ui.add_enabled(false, egui::Button::new("실행 취소"));
                ui.add_enabled(false, egui::Button::new("다시 실행"));
            });
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.heading("레이어");
                for layer in &self.project.drawing.layers {
                    ui.horizontal(|ui| {
                        let mut visible = layer.visible;
                        ui.add_enabled(false, egui::Checkbox::new(&mut visible, &layer.name));
                    });
                }
                ui.add_space(8.0);
                ui.weak("레이어 편집은 Phase 8c 예정");
            });
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(240.0)
            .show(ctx, |ui| {
                ui.heading("검증");
                let report = self.project.drawing.validate();
                if report.issues.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, "문제 없음");
                } else {
                    for issue in &report.issues {
                        ui.colored_label(egui::Color32::LIGHT_RED, &issue.message);
                        ui.weak(&issue.suggestion);
                    }
                }
                ui.add_space(8.0);
                ui.heading("속성");
                ui.weak("선택 객체 속성 편집은 Phase 8c 예정");
            });
    }

    /// Draws the canvas and handles pan (middle-drag) / zoom (scroll toward
    /// cursor). Returns the status line text for the bottom bar.
    fn canvas(&mut self, ctx: &egui::Context) -> String {
        let mut status = String::new();

        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = response.rect;
            let center = (rect.center().x, rect.center().y);

            if response.dragged_by(egui::PointerButton::Middle) {
                let delta = response.drag_delta();
                self.camera.pan((delta.x, delta.y));
            }

            if let Some(pointer) = response.hover_pos() {
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll.abs() > f32::EPSILON {
                    let factor = (1.0 + scroll * 0.001).clamp(0.5, 2.0);
                    self.camera.zoom_at(center, (pointer.x, pointer.y), factor);
                }

                let world = self.camera.screen_to_world(center, (pointer.x, pointer.y));
                status = format!(
                    "x: {:.2} mm, y: {:.2} mm   |   zoom: {:.2} px/mm",
                    world.0, world.1, self.camera.zoom
                );
            } else {
                status = format!("zoom: {:.2} px/mm", self.camera.zoom);
            }

            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(24));

            let primitives = cad_render::build_render_model(&self.project);
            for primitive in &primitives {
                draw_primitive(&painter, &self.camera, center, primitive);
            }
        });

        status
    }

    fn status_bar(&self, ctx: &egui::Context, status: &str) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(status);
                ui.separator();
                ui.label(format!(
                    "entities: {}   dimensions: {}",
                    self.project.drawing.entities.len(),
                    self.project.drawing.dimensions.len()
                ));
            });
        });
    }
}

fn draw_primitive(
    painter: &egui::Painter,
    camera: &Camera,
    center: (f32, f32),
    primitive: &RenderPrimitive,
) {
    let stroke = egui::Stroke::new(1.5, egui::Color32::from_gray(220));
    let dimension_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 170, 255));

    let to_screen = |p: (f64, f64)| {
        let (x, y) = camera.world_to_screen(center, p);
        egui::pos2(x, y)
    };

    match primitive {
        RenderPrimitive::Line { start, end } => {
            painter.line_segment([to_screen(*start), to_screen(*end)], stroke);
        }
        RenderPrimitive::Polyline { points, closed } => {
            let screen_points: Vec<egui::Pos2> = points.iter().map(|p| to_screen(*p)).collect();
            if *closed {
                painter.add(egui::Shape::closed_line(screen_points, stroke));
            } else {
                painter.add(egui::Shape::line(screen_points, stroke));
            }
        }
        RenderPrimitive::Rectangle {
            origin,
            width,
            height,
        } => {
            let corners = vec![
                to_screen(*origin),
                to_screen((origin.0 + width, origin.1)),
                to_screen((origin.0 + width, origin.1 + height)),
                to_screen((origin.0, origin.1 + height)),
            ];
            painter.add(egui::Shape::closed_line(corners, stroke));
        }
        RenderPrimitive::Circle { center: c, radius } => {
            let screen_center = to_screen(*c);
            let screen_radius = (*radius as f32) * camera.zoom;
            painter.circle_stroke(screen_center, screen_radius, stroke);
        }
        RenderPrimitive::Arc {
            center: c,
            radius,
            start_angle,
            sweep_angle,
        } => {
            const STEPS: u32 = 32;
            let points: Vec<egui::Pos2> = (0..=STEPS)
                .map(|i| {
                    let t = start_angle + sweep_angle * (f64::from(i) / f64::from(STEPS));
                    to_screen((c.0 + radius * t.cos(), c.1 + radius * t.sin()))
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
        }
        RenderPrimitive::Text {
            origin,
            content,
            height,
        } => {
            let screen_origin = to_screen(*origin);
            let font_size = ((*height as f32) * camera.zoom).max(4.0);
            painter.text(
                screen_origin,
                egui::Align2::LEFT_BOTTOM,
                content,
                egui::FontId::proportional(font_size),
                egui::Color32::WHITE,
            );
        }
        RenderPrimitive::Dimension { start, end, .. } => {
            painter.line_segment([to_screen(*start), to_screen(*end)], dimension_stroke);
        }
    }
}
