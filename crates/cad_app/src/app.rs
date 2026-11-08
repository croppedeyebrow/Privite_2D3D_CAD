// Screen-space drawing uses `f32` (egui's pixel type) for values sourced
// from `f64` world coordinates; see camera.rs for why the truncation is fine.
#![allow(clippy::cast_possible_truncation)]

use std::collections::HashSet;

use cad_command::{CommandHistory, DrawingCommand};
use cad_core::{
    AngleRad, Entity, EntityGeometry, EntityId, Group, GroupId, LayerId, LengthMm, Project,
    DEFAULT_LAYER_ID,
};
use cad_render::RenderPrimitive;

use crate::camera::Camera;
use crate::demo::seed_demo_project;
use crate::tool::{self, DrawState, Tool};

const PICK_TOLERANCE_PX: f32 = 8.0;

/// A working copy of the selected entity's geometry, edited by the
/// property panel's fields. Kept separate from `Project` so a multi-frame
/// drag on a `DragValue` has somewhere to accumulate before it's committed
/// as a single `SetEntityGeometry` (same reasoning as `DrawState::Moving`
/// in Phase 8b: one undo step per user gesture, not one per frame).
struct PropertyEdit {
    entity_id: EntityId,
    draft: EntityGeometry,
    /// True while a field is actively being dragged or has keyboard focus.
    /// While true, the draft is not overwritten by the live project value.
    editing: bool,
}

/// The layout from `03_프론트엔드_데스크톱_GUI_UX.md` 2절 (기본 화면) with the
/// Drafting workbench's panels (Layers / Properties / Validation, per the
/// 워크벤치 화면별 계약 table): menu, tool bar, left (layers), center
/// (canvas), right (validation + properties), bottom (status).
/// Drawing, multi-selection (click, shift-click, box select), grouping,
/// movement, deletion, undo/redo, and property/layer editing all work.
/// Still missing: file open/save from the GUI, and the dimension tool
/// (its placement rules are not settled yet — see Phase 5·6 notes).
pub struct CadApp {
    project: Project,
    camera: Camera,
    history: CommandHistory,
    tool: Tool,
    draw_state: DrawState,
    /// The current selection. A `HashSet` rather than a `Vec` because
    /// membership tests (`contains`) happen every frame while dragging or
    /// box-selecting, and duplicates would be meaningless anyway.
    selection: HashSet<EntityId>,
    property_edit: Option<PropertyEdit>,
    current_layer: LayerId,
    next_entity_id: u64,
    next_group_id: u64,
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
        let next_group_id = project
            .drawing
            .groups
            .iter()
            .map(|g| g.id.value())
            .max()
            .map_or(1, |max| max + 1);

        Self {
            project,
            camera: Camera::default(),
            history: CommandHistory::default(),
            tool: Tool::Select,
            draw_state: DrawState::Idle,
            selection: HashSet::new(),
            property_edit: None,
            current_layer: DEFAULT_LAYER_ID,
            next_entity_id,
            next_group_id,
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
        let (undo_pressed, redo_pressed, escape_pressed, delete_pressed) = ctx.input(|i| {
            let undo = i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift;
            let redo = (i.key_pressed(egui::Key::Y) && i.modifiers.command)
                || (i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift);
            (
                undo,
                redo,
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Delete),
            )
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
        if delete_pressed {
            self.delete_selection();
        }
    }

    /// Deletes every selected entity. Each deletion is its own undo step
    /// (there is no batch `DeleteEntities` command yet) — undoing a bulk
    /// delete brings entities back one at a time rather than all at once.
    fn delete_selection(&mut self) {
        for id in std::mem::take(&mut self.selection) {
            self.execute(DrawingCommand::DeleteEntity { id });
        }
    }

    /// Whether the current selection is eligible to become a new group:
    /// two or more entities, none of which already belong to a group
    /// (grouping an already-grouped entity into a second group isn't
    /// supported — ungroup it first).
    fn can_group_selection(&self) -> bool {
        self.selection.len() >= 2
            && self
                .selection
                .iter()
                .all(|id| self.project.drawing.group_containing(*id).is_none())
    }

    /// The group the entire selection belongs to, if every selected entity
    /// shares the same one. `None` if the selection is empty, ungrouped, or
    /// spans more than one group.
    fn selection_group_id(&self) -> Option<GroupId> {
        let mut ids = self
            .selection
            .iter()
            .map(|id| self.project.drawing.group_containing(*id).map(|g| g.id));
        let first = ids.next()?;
        if ids.all(|next| next == first) {
            first
        } else {
            None
        }
    }

    fn group_selection(&mut self) {
        if !self.can_group_selection() {
            return;
        }
        let id = GroupId::new(self.next_group_id);
        self.next_group_id += 1;
        let group = Group {
            id,
            name: format!("그룹 {}", id.value()),
            member_ids: self.selection.iter().copied().collect(),
        };
        self.execute(DrawingCommand::GroupEntities(group));
    }

    fn ungroup_selection(&mut self) {
        if let Some(group_id) = self.selection_group_id() {
            self.execute(DrawingCommand::UngroupEntities { group_id });
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
                    .add_enabled(self.can_group_selection(), egui::Button::new("그룹"))
                    .clicked()
                {
                    self.group_selection();
                }
                if ui
                    .add_enabled(
                        self.selection_group_id().is_some(),
                        egui::Button::new("그룹 해제"),
                    )
                    .clicked()
                {
                    self.ungroup_selection();
                }

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
        let layers = self.project.drawing.layers.clone();
        let current_layer = self.current_layer;
        let mut visibility_toggle: Option<(LayerId, bool)> = None;
        let mut lock_toggle: Option<(LayerId, bool)> = None;
        let mut new_current_layer: Option<LayerId> = None;

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("레이어");
                ui.weak("이름을 클릭하면 그리기 대상 레이어로 지정됩니다.");
                for layer in &layers {
                    ui.horizontal(|ui| {
                        let mut visible = layer.visible;
                        if ui.checkbox(&mut visible, "").changed() {
                            visibility_toggle = Some((layer.id, visible));
                        }
                        let mut locked = layer.locked;
                        if ui.checkbox(&mut locked, "🔒").changed() {
                            lock_toggle = Some((layer.id, locked));
                        }
                        if ui
                            .selectable_label(current_layer == layer.id, &layer.name)
                            .clicked()
                        {
                            new_current_layer = Some(layer.id);
                        }
                    });
                }
            });

        // Executed after `.show` returns so these don't need `&mut self`
        // while the panel closure above is (immutably) reading `layers`.
        if let Some((id, visible)) = visibility_toggle {
            self.execute(DrawingCommand::SetLayerVisible { id, visible });
        }
        if let Some((id, locked)) = lock_toggle {
            self.execute(DrawingCommand::SetLayerLocked { id, locked });
        }
        if let Some(id) = new_current_layer {
            self.current_layer = id;
        }
    }

    /// The selected entity when exactly one is selected. The property panel
    /// edits a single entity's geometry, so a multi-selection (or an empty
    /// one) has nothing to show.
    fn single_selection(&self) -> Option<EntityId> {
        let mut ids = self.selection.iter().copied();
        let only = ids.next()?;
        ids.next().is_none().then_some(only)
    }

    /// Keeps `property_edit` pointed at the current selection. Does nothing
    /// while a field is actively being dragged/typed into, so an in-progress
    /// edit is never clobbered by a fresh clone of the (not-yet-updated)
    /// project entity.
    fn sync_property_edit(&mut self) {
        let Some(id) = self.single_selection() else {
            self.property_edit = None;
            return;
        };

        let currently_editing = self
            .property_edit
            .as_ref()
            .is_some_and(|edit| edit.entity_id == id && edit.editing);
        if currently_editing {
            return;
        }

        self.property_edit = self
            .project
            .drawing
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .map(|entity| PropertyEdit {
                entity_id: id,
                draft: entity.geometry.clone(),
                editing: false,
            });
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        self.sync_property_edit();

        let report = self.project.drawing.validate();
        let status_message = self.status_message.clone();
        let selected = self.single_selection().and_then(|id| {
            self.project
                .drawing
                .entities
                .iter()
                .find(|entity| entity.id == id)
                .map(|entity| (entity.id, geometry_kind_label(&entity.geometry)))
        });
        let multi_selection_count = (self.selection.len() > 1).then_some(self.selection.len());
        let mut draft = self.property_edit.as_ref().map(|edit| edit.draft.clone());
        let mut outcome = FieldEditOutcome::default();

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("검증");
                if report.issues.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, "문제 없음");
                } else {
                    for issue in &report.issues {
                        ui.colored_label(egui::Color32::LIGHT_RED, &issue.message);
                        ui.weak(&issue.suggestion);
                    }
                }
                if let Some(message) = &status_message {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::LIGHT_RED, message);
                }

                ui.add_space(8.0);
                ui.heading("속성");
                match (&selected, &mut draft) {
                    (Some((id, kind)), Some(geometry)) => {
                        ui.label(format!("id: {id}"));
                        ui.label(format!("종류: {kind}"));
                        ui.separator();
                        outcome = edit_geometry_fields(ui, geometry);
                    }
                    _ => {
                        if let Some(count) = multi_selection_count {
                            ui.weak(format!(
                                "{count}개 선택됨 — 속성 편집은 하나만 선택했을 때 가능합니다"
                            ));
                        } else {
                            ui.weak("선택된 엔티티 없음");
                        }
                    }
                }
            });

        let (Some((id, _)), Some(geometry)) = (selected, draft) else {
            return;
        };
        if outcome.commit {
            self.execute(DrawingCommand::SetEntityGeometry { id, geometry });
            if let Some(edit) = &mut self.property_edit {
                edit.editing = false;
            }
        } else if let Some(edit) = &mut self.property_edit {
            edit.draft = geometry;
            edit.editing = outcome.editing;
        }
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
            self.draw_box_select(&painter, center, hover_world);

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
                if let Some(snapped) = snapped_hover {
                    if let Some(text) = tool::preview_measurement(&self.draw_state, snapped) {
                        let (x, y) = self.camera.world_to_screen(center, snapped);
                        painter.text(
                            egui::pos2(x + 14.0, y - 14.0),
                            egui::Align2::LEFT_BOTTOM,
                            text,
                            egui::FontId::proportional(14.0),
                            egui::Color32::from_rgb(255, 210, 90),
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
        let additive = response.ctx.input(|i| i.modifiers.shift);

        if response.drag_started() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let world = self.camera.screen_to_world(center, (pointer.x, pointer.y));
                match tool::hit_test(&self.project.drawing, world, tolerance_mm) {
                    // Pressing on an entity drags the whole selection it
                    // belongs to, so a multi-selection moves together.
                    Some(id) => {
                        self.apply_pick(id, additive);
                        self.draw_state = DrawState::Moving {
                            entity_ids: self.selection.iter().copied().collect(),
                            screen_delta: (0.0, 0.0),
                        };
                    }
                    // Pressing on empty canvas starts a rubber-band box.
                    None => self.draw_state = DrawState::BoxSelect { anchor: world },
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
            match std::mem::take(&mut self.draw_state) {
                DrawState::Moving {
                    entity_ids,
                    screen_delta,
                } => {
                    if screen_delta.0.abs() > f32::EPSILON || screen_delta.1.abs() > f32::EPSILON {
                        let dx = f64::from(screen_delta.0 / self.camera.zoom);
                        let dy = f64::from(-screen_delta.1 / self.camera.zoom);
                        self.execute(DrawingCommand::MoveEntities {
                            ids: entity_ids,
                            dx: LengthMm(dx),
                            dy: LengthMm(dy),
                        });
                    }
                }
                DrawState::BoxSelect { anchor } => {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let world = self.camera.screen_to_world(center, (pointer.x, pointer.y));
                        let picked = tool::entities_in_box(&self.project.drawing, anchor, world);
                        if !additive {
                            self.selection.clear();
                        }
                        // Box-selecting part of a group selects all of it,
                        // matching what clicking one member does.
                        for id in picked {
                            self.selection
                                .extend(tool::expand_group(&self.project.drawing, id));
                        }
                    }
                }
                // Any other state was not a selection gesture; `take` above
                // already reset it to `Idle`.
                _ => {}
            }
        } else if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let world = self.camera.screen_to_world(center, (pointer.x, pointer.y));
                match tool::hit_test(&self.project.drawing, world, tolerance_mm) {
                    Some(id) => self.apply_pick(id, additive),
                    None if !additive => self.selection.clear(),
                    None => {}
                }
            }
        }
    }

    /// Applies a single pick to the selection. A picked entity brings its
    /// whole group along ([`tool::expand_group`]). Without `additive` the
    /// pick replaces the selection; with it, an already-selected group is
    /// removed and an unselected one is added (shift-click toggles).
    fn apply_pick(&mut self, id: EntityId, additive: bool) {
        let picked = tool::expand_group(&self.project.drawing, id);
        if !additive {
            // Re-picking inside the current selection keeps it intact, so
            // dragging a multi-selection doesn't collapse it to one entity.
            if !self.selection.is_superset(&picked) {
                self.selection = picked;
            }
            return;
        }
        if self.selection.is_superset(&picked) {
            self.selection.retain(|id| !picked.contains(id));
        } else {
            self.selection.extend(picked);
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
        if self.selection.is_empty() {
            return;
        }
        let highlight_stroke = egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 200, 60));
        // Entities being dragged are drawn at a pixel offset for live
        // preview; the project itself is only changed when the drag ends.
        let moving = match &self.draw_state {
            DrawState::Moving {
                entity_ids,
                screen_delta,
            } => Some((entity_ids, *screen_delta)),
            _ => None,
        };

        for entity in &self.project.drawing.entities {
            if !self.selection.contains(&entity.id) {
                continue;
            }
            let offset = moving.map_or((0.0, 0.0), |(ids, delta)| {
                if ids.contains(&entity.id) {
                    delta
                } else {
                    (0.0, 0.0)
                }
            });
            draw_geometry(
                painter,
                &self.camera,
                center,
                &entity.geometry,
                offset,
                highlight_stroke,
            );
        }
    }

    /// Draws the rubber-band rectangle while a box selection is in progress.
    fn draw_box_select(
        &self,
        painter: &egui::Painter,
        center: (f32, f32),
        hover_world: Option<(f64, f64)>,
    ) {
        let DrawState::BoxSelect { anchor } = self.draw_state else {
            return;
        };
        let Some(hover) = hover_world else { return };
        let (ax, ay) = self.camera.world_to_screen(center, anchor);
        let (bx, by) = self.camera.world_to_screen(center, hover);
        painter.rect_stroke(
            egui::Rect::from_two_pos(egui::pos2(ax, ay), egui::pos2(bx, by)),
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(140, 190, 255)),
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

/// Whether the property panel's edits this frame should be committed as a
/// `SetEntityGeometry`, and whether a field is still mid-interaction (see
/// `PropertyEdit::editing`).
#[derive(Default)]
struct FieldEditOutcome {
    commit: bool,
    editing: bool,
}

fn number_field(ui: &mut egui::Ui, outcome: &mut FieldEditOutcome, label: &str, value: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        let response = ui.add(egui::DragValue::new(value).speed(0.5).suffix(" mm"));
        if response.dragged() || response.has_focus() {
            outcome.editing = true;
        }
        if response.drag_stopped() || response.lost_focus() {
            outcome.commit = true;
        }
    });
}

fn angle_field_degrees(
    ui: &mut egui::Ui,
    outcome: &mut FieldEditOutcome,
    label: &str,
    angle: &mut AngleRad,
) {
    let mut degrees = angle.0.to_degrees();
    ui.horizontal(|ui| {
        ui.label(label);
        let response = ui.add(egui::DragValue::new(&mut degrees).speed(1.0).suffix("°"));
        if response.dragged() || response.has_focus() {
            outcome.editing = true;
        }
        if response.drag_stopped() || response.lost_focus() {
            outcome.commit = true;
        }
    });
    angle.0 = degrees.to_radians();
}

/// Renders editable fields for whichever geometry variant is selected.
/// Every field follows the same rule as canvas dragging (Phase 8b): the
/// displayed value updates live, but `outcome.commit` — and therefore the
/// `SetEntityGeometry` command — only fires once the drag/typing ends, so
/// one property edit is one undo step.
fn edit_geometry_fields(ui: &mut egui::Ui, geometry: &mut EntityGeometry) -> FieldEditOutcome {
    let mut outcome = FieldEditOutcome::default();

    match geometry {
        EntityGeometry::Line(line) => {
            number_field(ui, &mut outcome, "시작 x", &mut line.start.x.0);
            number_field(ui, &mut outcome, "시작 y", &mut line.start.y.0);
            number_field(ui, &mut outcome, "끝 x", &mut line.end.x.0);
            number_field(ui, &mut outcome, "끝 y", &mut line.end.y.0);
        }
        EntityGeometry::Rectangle(rect) => {
            number_field(ui, &mut outcome, "원점 x", &mut rect.origin.x.0);
            number_field(ui, &mut outcome, "원점 y", &mut rect.origin.y.0);
            number_field(ui, &mut outcome, "폭", &mut rect.width.0);
            number_field(ui, &mut outcome, "높이", &mut rect.height.0);
        }
        EntityGeometry::Circle(circle) => {
            number_field(ui, &mut outcome, "중심 x", &mut circle.center.x.0);
            number_field(ui, &mut outcome, "중심 y", &mut circle.center.y.0);
            number_field(ui, &mut outcome, "반지름", &mut circle.radius.0);
        }
        EntityGeometry::Arc(arc) => {
            number_field(ui, &mut outcome, "중심 x", &mut arc.center.x.0);
            number_field(ui, &mut outcome, "중심 y", &mut arc.center.y.0);
            number_field(ui, &mut outcome, "반지름", &mut arc.radius.0);
            angle_field_degrees(ui, &mut outcome, "시작각", &mut arc.start_angle);
            angle_field_degrees(ui, &mut outcome, "스윕각", &mut arc.sweep_angle);
        }
        EntityGeometry::Text(text) => {
            ui.horizontal(|ui| {
                ui.label("내용");
                let response = ui.text_edit_singleline(&mut text.content);
                if response.has_focus() {
                    outcome.editing = true;
                }
                if response.lost_focus() {
                    outcome.commit = true;
                }
            });
            number_field(ui, &mut outcome, "원점 x", &mut text.origin.x.0);
            number_field(ui, &mut outcome, "원점 y", &mut text.origin.y.0);
            number_field(ui, &mut outcome, "높이", &mut text.height.0);
        }
        EntityGeometry::Polyline(_) => {
            ui.weak("폴리라인 속성 편집은 아직 지원하지 않습니다.");
        }
    }

    outcome
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

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{Line, Point2};

    /// A three-entity app with entities 1 and 2 in a group, entity 3 loose.
    /// Built without any `egui` context — the selection helpers under test
    /// only read `project.drawing` and write `selection`.
    fn app_with_group() -> CadApp {
        let mut app = CadApp::default();
        let mut project = Project::default();
        for (id, offset) in [(1u64, 10.0), (2, 20.0), (3, 30.0)] {
            project
                .drawing
                .add_entity(Entity {
                    id: EntityId::new(id),
                    layer_id: DEFAULT_LAYER_ID,
                    geometry: EntityGeometry::Line(Line {
                        start: Point2::new(offset, 0.0),
                        end: Point2::new(offset, 5.0),
                    }),
                })
                .unwrap();
        }
        project
            .drawing
            .add_group(Group {
                id: GroupId::new(1),
                name: "그룹 1".to_owned(),
                member_ids: vec![EntityId::new(1), EntityId::new(2)],
            })
            .unwrap();
        app.project = project;
        app
    }

    fn ids(values: &[u64]) -> HashSet<EntityId> {
        values.iter().copied().map(EntityId::new).collect()
    }

    #[test]
    fn plain_pick_replaces_the_selection() {
        let mut app = app_with_group();
        app.apply_pick(EntityId::new(3), false);
        assert_eq!(app.selection, ids(&[3]));
    }

    #[test]
    fn picking_a_grouped_entity_selects_the_whole_group() {
        let mut app = app_with_group();
        app.apply_pick(EntityId::new(1), false);
        assert_eq!(app.selection, ids(&[1, 2]));
    }

    #[test]
    fn plain_pick_inside_the_current_selection_keeps_it_intact() {
        // Otherwise pressing on one member of a multi-selection to drag it
        // would collapse the selection to that single entity.
        let mut app = app_with_group();
        app.selection = ids(&[1, 2, 3]);
        app.apply_pick(EntityId::new(3), false);
        assert_eq!(app.selection, ids(&[1, 2, 3]));
    }

    #[test]
    fn additive_pick_adds_then_removes_the_same_group() {
        let mut app = app_with_group();
        app.apply_pick(EntityId::new(3), true);
        app.apply_pick(EntityId::new(2), true);
        assert_eq!(app.selection, ids(&[1, 2, 3]));

        // Shift-clicking the group again toggles all of its members off.
        app.apply_pick(EntityId::new(1), true);
        assert_eq!(app.selection, ids(&[3]));
    }

    #[test]
    fn single_selection_is_none_unless_exactly_one_is_selected() {
        let mut app = app_with_group();
        assert_eq!(app.single_selection(), None);

        app.selection = ids(&[3]);
        assert_eq!(app.single_selection(), Some(EntityId::new(3)));

        app.selection = ids(&[1, 2]);
        assert_eq!(app.single_selection(), None);
    }

    #[test]
    fn delete_selection_removes_every_selected_entity() {
        let mut app = app_with_group();
        app.selection = ids(&[1, 2]);
        app.delete_selection();
        assert!(app.selection.is_empty());
        assert_eq!(app.project.drawing.entities.len(), 1);
        assert_eq!(app.project.drawing.entities[0].id, EntityId::new(3));
    }
}
