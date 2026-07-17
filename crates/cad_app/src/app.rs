// Screen-space drawing uses `f32` (egui's pixel type) for values sourced
// from `f64` world coordinates; see camera.rs for why the truncation is fine.
#![allow(clippy::cast_possible_truncation)]

use cad_command::{CommandHistory, DrawingCommand};
use cad_core::{Entity, EntityGeometry, EntityId, LayerId, LengthMm, Project, DEFAULT_LAYER_ID};
use cad_render::RenderPrimitive;

use crate::camera::Camera;
use crate::demo::seed_demo_project;
use crate::tool::{self, DrawState, Tool};

const PICK_TOLERANCE_PX: f32 = 8.0;

/// The `04_UI_와이어프레임_정책.md` layout, now with working tools: select,
/// draw five entity shapes, move the selection, undo/redo. File operations
/// and property editing are still placeholders — Phase 8c/8d.
pub struct CadApp {
    project: Project,
    camera: Camera,
    history: CommandHistory,
    tool: Tool,
    draw_state: DrawState,
    selection: Option<EntityId>,
    current_layer: LayerId,
    next_entity_id: u64,
    status_message: Option<String>,
}

impl Default for CadApp {
    fn default() -> Self {
        let project = seed_demo_project();
        let next_entity_id = project
            .drawing
            .entities
            .iter()
            .map(|e| e.id.value())
            .max()
            .map_or(1, |max| max + 1);

        Self {
            project,
            camera: Camera::default(),
            history: CommandHistory::default(),
            tool: Tool::Select,
            draw_state: DrawState::Idle,
            selection: None,
            current_layer: DEFAULT_LAYER_ID,
            next_entity_id,
            status_message: None,
        }
    }
}

impl eframe::App for CadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.menu_bar(ctx);
        self.tool_bar(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        let status = self.canvas(ctx);
        self.status_bar(ctx, &status);
    }
}

impl CadApp {
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (undo_pressed, redo_pressed, escape_pressed) = ctx.input(|i| {
            let undo = i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift;
            let redo = (i.key_pressed(egui::Key::Y) && i.modifiers.command)
                || (i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift);
            (undo, redo, i.key_pressed(egui::Key::Escape))
        });
        if undo_pressed {
            self.undo();
        }
        if redo_pressed {
            self.redo();
        }
        if escape_pressed {
            self.draw_state = DrawState::Idle;
        }
    }

    fn execute(&mut self, command: DrawingCommand) {
        match self.history.execute(&mut self.project, command) {
            Ok(()) => self.status_message = None,
            Err(err) => self.status_message = Some(err.to_string()),
        }
    }

    fn undo(&mut self) {
        match self.history.undo(&mut self.project) {
            Ok(_) => self.status_message = None,
            Err(err) => self.status_message = Some(err.to_string()),
        }
    }

    fn redo(&mut self) {
        match self.history.redo(&mut self.project) {
            Ok(_) => self.status_message = None,
            Err(err) => self.status_message = Some(err.to_string()),
        }
    }

    fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            self.tool = tool;
            self.draw_state = DrawState::Idle;
        }
    }

    fn alloc_entity_id(&mut self) -> EntityId {
        let id = EntityId::new(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }

    fn add_entity(&mut self, geometry: EntityGeometry) {
        let id = self.alloc_entity_id();
        let entity = Entity {
            id,
            layer_id: self.current_layer,
            geometry,
        };
        self.execute(DrawingCommand::AddEntity(entity));
    }

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("파일", |ui| {
                    ui.weak("새 프로젝트 (Phase 8d 예정)");
                    ui.weak("열기 (Phase 8d 예정)");
                    ui.weak("저장 (Phase 8d 예정)");
                });
                ui.menu_button("편집", |ui| {
                    if ui
                        .add_enabled(self.history.can_undo(), egui::Button::new("실행 취소"))
                        .clicked()
                    {
                        self.undo();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.history.can_redo(), egui::Button::new("다시 실행"))
                        .clicked()
                    {
                        self.redo();
                        ui.close_menu();
                    }
                });
                ui.menu_button("보기", |ui| {
                    if ui.button("보기 초기화").clicked() {
                        self.camera = Camera::default();
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn tool_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tool_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.tool == Tool::Select, Tool::Select.label())
                    .clicked()
                {
                    self.set_tool(Tool::Select);
                }
                for tool in Tool::DRAWING_TOOLS {
                    if ui
                        .selectable_label(self.tool == tool, tool.label())
                        .clicked()
                    {
                        self.set_tool(tool);
                    }
                }
                ui.add_enabled(false, egui::Button::new("치수")); // Phase 8d

                ui.separator();
                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("실행 취소"))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("다시 실행"))
                    .clicked()
                {
                    self.redo();
                }
            });
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.heading("레이어");
                ui.weak("클릭하면 그리기 대상 레이어로 지정됩니다.");
                for layer in self.project.drawing.layers.clone() {
                    ui.horizontal(|ui| {
                        let mut visible = layer.visible;
                        ui.add_enabled(false, egui::Checkbox::new(&mut visible, ""));
                        if ui
                            .selectable_label(self.current_layer == layer.id, &layer.name)
                            .clicked()
                        {
                            self.current_layer = layer.id;
                        }
                    });
                }
                ui.add_space(8.0);
                ui.weak("가시성/잠금 편집은 Phase 8c 예정");
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
                if let Some(message) = &self.status_message {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::LIGHT_RED, message);
                }

                ui.add_space(8.0);
                ui.heading("속성");
                match self.selection.and_then(|id| {
                    self.project
                        .drawing
                        .entities
                        .iter()
                        .find(|entity| entity.id == id)
                }) {
                    Some(entity) => {
                        ui.label(format!("id: {}", entity.id));
                        ui.label(format!("종류: {}", geometry_kind_label(&entity.geometry)));
                        ui.weak("값 편집은 Phase 8c 예정");
                    }
                    None => {
                        ui.weak("선택된 엔티티 없음");
                    }
                }
            });
    }

    /// Draws the canvas, handles pan/zoom/tool interaction, and returns the
    /// status line text for the bottom bar.
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
            }

            let tolerance_mm = f64::from(PICK_TOLERANCE_PX / self.camera.zoom);
            let hover_world = response
                .hover_pos()
                .map(|p| self.camera.screen_to_world(center, (p.x, p.y)));
            let snapped_hover =
                hover_world.map(|w| tool::snap_point(&self.project.drawing, w, tolerance_mm));

            status = self.status_line(snapped_hover);

            match self.tool {
                Tool::Select => self.handle_select_interaction(&response, center, tolerance_mm),
                _ => self.handle_drawing_interaction(&response, center, tolerance_mm),
            }

            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(24));

            let default_stroke = egui::Stroke::new(1.5, egui::Color32::from_gray(220));
            let dimension_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 170, 255));
            let primitives = cad_render::build_render_model(&self.project);
            for primitive in &primitives {
                let stroke = if matches!(primitive, RenderPrimitive::Dimension { .. }) {
                    dimension_stroke
                } else {
                    default_stroke
                };
                draw_primitive(
                    &painter,
                    &self.camera,
                    center,
                    (0.0, 0.0),
                    stroke,
                    primitive,
                );
            }

            self.draw_selection_highlight(&painter, center);

            if !matches!(self.tool, Tool::Select) {
                draw_tool_preview(
                    &painter,
                    &self.camera,
                    center,
                    &self.draw_state,
                    snapped_hover,
                );
                if let (Some(raw), Some(snapped)) = (hover_world, snapped_hover) {
                    if raw != snapped {
                        let (x, y) = self.camera.world_to_screen(center, snapped);
                        painter.circle_stroke(
                            egui::pos2(x, y),
                            5.0,
                            egui::Stroke::new(1.5, egui::Color32::YELLOW),
                        );
                    }
                }
            }
        });

        self.text_input_popup(ctx);

        status
    }

    fn status_line(&self, snapped_hover: Option<(f64, f64)>) -> String {
        let position = snapped_hover.map_or_else(String::new, |world| {
            format!("x: {:.2} mm, y: {:.2} mm   |   ", world.0, world.1)
        });
        format!(
            "{position}zoom: {:.2} px/mm   |   tool: {}",
            self.camera.zoom,
            self.tool.label()
        )
    }

    fn handle_select_interaction(
        &mut self,
        response: &egui::Response,
        center: (f32, f32),
        tolerance_mm: f64,
    ) {
        if response.drag_started() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let world = self.camera.screen_to_world(center, (pointer.x, pointer.y));
                self.selection = tool::hit_test(&self.project.drawing, world, tolerance_mm);
                if let Some(id) = self.selection {
                    self.draw_state = DrawState::Moving {
                        entity_id: id,
                        screen_delta: (0.0, 0.0),
                    };
                }
            }
        }

        if response.dragged() {
            if let DrawState::Moving { screen_delta, .. } = &mut self.draw_state {
                let delta = response.drag_delta();
                screen_delta.0 += delta.x;
                screen_delta.1 += delta.y;
            }
        }

        if response.drag_stopped() {
            if let DrawState::Moving {
                entity_id,
                screen_delta,
            } = self.draw_state
            {
                self.draw_state = DrawState::Idle;
                if screen_delta.0.abs() > f32::EPSILON || screen_delta.1.abs() > f32::EPSILON {
                    let dx = f64::from(screen_delta.0 / self.camera.zoom);
                    let dy = f64::from(-screen_delta.1 / self.camera.zoom);
                    self.execute(DrawingCommand::MoveEntity {
                        id: entity_id,
                        dx: LengthMm(dx),
                        dy: LengthMm(dy),
                    });
                }
            }
        } else if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let world = self.camera.screen_to_world(center, (pointer.x, pointer.y));
                self.selection = tool::hit_test(&self.project.drawing, world, tolerance_mm);
            }
        }
    }

    fn handle_drawing_interaction(
        &mut self,
        response: &egui::Response,
        center: (f32, f32),
        tolerance_mm: f64,
    ) {
        if !response.clicked() {
            return;
        }
        let Some(pointer) = response.interact_pointer_pos() else {
            return;
        };
        let world = self.camera.screen_to_world(center, (pointer.x, pointer.y));
        let point = tool::snap_point(&self.project.drawing, world, tolerance_mm);

        match (self.tool, self.draw_state.clone()) {
            (Tool::Line, DrawState::Idle) => self.draw_state = DrawState::LineStart(point),
            (Tool::Line, DrawState::LineStart(start)) => {
                self.draw_state = DrawState::Idle;
                self.add_entity(tool::line_geometry(start, point));
            }
            (Tool::Rectangle, DrawState::Idle) => {
                self.draw_state = DrawState::RectangleStart(point);
            }
            (Tool::Rectangle, DrawState::RectangleStart(start)) => {
                self.draw_state = DrawState::Idle;
                self.add_entity(tool::rectangle_geometry(start, point));
            }
            (Tool::Circle, DrawState::Idle) => self.draw_state = DrawState::CircleCenter(point),
            (Tool::Circle, DrawState::CircleCenter(circle_center)) => {
                self.draw_state = DrawState::Idle;
                self.add_entity(tool::circle_geometry(circle_center, point));
            }
            (Tool::Arc, DrawState::Idle) => self.draw_state = DrawState::ArcCenter(point),
            (Tool::Arc, DrawState::ArcCenter(arc_center)) => {
                self.draw_state = DrawState::ArcStart {
                    center: arc_center,
                    start: point,
                };
            }
            (Tool::Arc, DrawState::ArcStart { center: c, start }) => {
                self.draw_state = DrawState::Idle;
                self.add_entity(tool::arc_geometry(c, start, point));
            }
            (Tool::Text, DrawState::Idle) => {
                self.draw_state = DrawState::TextPending {
                    origin: point,
                    content: String::new(),
                };
            }
            _ => {}
        }
    }

    fn draw_selection_highlight(&self, painter: &egui::Painter, center: (f32, f32)) {
        let Some(id) = self.selection else { return };
        let Some(entity) = self
            .project
            .drawing
            .entities
            .iter()
            .find(|entity| entity.id == id)
        else {
            return;
        };
        let offset = match self.draw_state {
            DrawState::Moving {
                entity_id,
                screen_delta,
            } if entity_id == id => screen_delta,
            _ => (0.0, 0.0),
        };
        let highlight_stroke = egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 200, 60));
        draw_geometry(
            painter,
            &self.camera,
            center,
            &entity.geometry,
            offset,
            highlight_stroke,
        );
    }

    /// Shows a floating text box near the pending text origin. `egui`'s
    /// immediate-mode model means the widget's current value must be read
    /// back into `draw_state` every frame it's open, but writing that back
    /// requires `&mut self` — so state is taken by value up front and the
    /// borrow of `self.draw_state` never overlaps the later `&mut self`
    /// calls (`add_entity`) used to commit it.
    fn text_input_popup(&mut self, ctx: &egui::Context) {
        let DrawState::TextPending { origin, content } = self.draw_state.clone() else {
            return;
        };

        let mut content = content;
        let mut confirmed = false;
        let mut cancelled = false;

        egui::Window::new("텍스트 입력")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let response = ui.text_edit_singleline(&mut content);
                response.request_focus();
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirmed = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("확인").clicked() {
                        confirmed = true;
                    }
                    if ui.button("취소").clicked() {
                        cancelled = true;
                    }
                });
            });

        if confirmed && !content.trim().is_empty() {
            self.draw_state = DrawState::Idle;
            self.add_entity(tool::text_geometry(origin, content));
        } else if cancelled {
            self.draw_state = DrawState::Idle;
        } else {
            self.draw_state = DrawState::TextPending { origin, content };
        }
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

fn geometry_kind_label(geometry: &EntityGeometry) -> &'static str {
    match geometry {
        EntityGeometry::Line(_) => "선",
        EntityGeometry::Polyline(_) => "폴리라인",
        EntityGeometry::Rectangle(_) => "사각형",
        EntityGeometry::Circle(_) => "원",
        EntityGeometry::Arc(_) => "호",
        EntityGeometry::Text(_) => "텍스트",
    }
}

fn draw_geometry(
    painter: &egui::Painter,
    camera: &Camera,
    center: (f32, f32),
    geometry: &EntityGeometry,
    offset: (f32, f32),
    stroke: egui::Stroke,
) {
    let primitive = cad_render::geometry_primitive(geometry);
    draw_primitive(painter, camera, center, offset, stroke, &primitive);
}

fn draw_tool_preview(
    painter: &egui::Painter,
    camera: &Camera,
    center: (f32, f32),
    draw_state: &DrawState,
    hover: Option<(f64, f64)>,
) {
    let Some(hover) = hover else { return };
    let preview_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 210, 90));

    let geometry = match draw_state {
        DrawState::LineStart(start) => Some(tool::line_geometry(*start, hover)),
        DrawState::RectangleStart(start) => Some(tool::rectangle_geometry(*start, hover)),
        DrawState::CircleCenter(circle_center) => {
            Some(tool::circle_geometry(*circle_center, hover))
        }
        DrawState::ArcCenter(arc_center) => Some(tool::line_geometry(*arc_center, hover)),
        DrawState::ArcStart { center: c, start } => Some(tool::arc_geometry(*c, *start, hover)),
        _ => None,
    };

    if let Some(geometry) = geometry {
        draw_geometry(
            painter,
            camera,
            center,
            &geometry,
            (0.0, 0.0),
            preview_stroke,
        );
    }
}

fn draw_primitive(
    painter: &egui::Painter,
    camera: &Camera,
    center: (f32, f32),
    offset: (f32, f32),
    stroke: egui::Stroke,
    primitive: &RenderPrimitive,
) {
    let to_screen = |p: (f64, f64)| {
        let (x, y) = camera.world_to_screen(center, p);
        egui::pos2(x + offset.0, y + offset.1)
    };

    match primitive {
        RenderPrimitive::Line { start, end } | RenderPrimitive::Dimension { start, end, .. } => {
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
    }
}
