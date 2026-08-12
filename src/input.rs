use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    W,
    A,
    S,
    D,
    Space,
    Shift,
    Ctrl,
    Escape,
    Tab,
    F1,
    F2,
    F3,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    M,
    T,
    G,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Resource)]
pub struct InputState {
    pub keys_pressed: HashSet<KeyCode>,
    pub keys_just_pressed: HashSet<KeyCode>,
    pub keys_just_released: HashSet<KeyCode>,
    pub mouse_position: Vec2,
    pub mouse_world_position: Vec2,
    pub mouse_buttons: HashSet<MouseButton>,
    pub mouse_just_pressed: HashSet<MouseButton>,
    pub mouse_just_released: HashSet<MouseButton>,
    pub scroll_delta: f32,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_position: Vec2::ZERO,
            mouse_world_position: Vec2::ZERO,
            mouse_buttons: HashSet::new(),
            mouse_just_pressed: HashSet::new(),
            mouse_just_released: HashSet::new(),
            scroll_delta: 0.0,
        }
    }
}

impl InputState {
    pub fn clear_frame_state(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
        self.scroll_delta = 0.0;
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons.contains(&button)
    }

    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_just_pressed.contains(&button)
    }
}

#[derive(Component)]
pub struct Selectable {
    pub selected: bool,
    pub hover: bool,
    pub radius: f32,
}

#[derive(Resource)]
pub struct SelectionManager {
    pub selected_entities: Vec<Entity>,
    pub selection_box_start: Option<Vec2>,
    pub selection_box_end: Option<Vec2>,
}

impl Default for SelectionManager {
    fn default() -> Self {
        Self {
            selected_entities: Vec::new(),
            selection_box_start: None,
            selection_box_end: None,
        }
    }
}

impl SelectionManager {
    pub fn clear(&mut self) {
        self.selected_entities.clear();
    }

    pub fn add(&mut self, entity: Entity) {
        if !self.selected_entities.contains(&entity) {
            self.selected_entities.push(entity);
        }
    }

    pub fn remove(&mut self, entity: Entity) {
        self.selected_entities.retain(|&e| e != entity);
    }

    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }

    pub fn count(&self) -> usize {
        self.selected_entities.len()
    }
}

pub fn input_clear_system(mut input: ResMut<InputState>) {
    input.clear_frame_state();
}

pub fn selection_input_system(
    input: Res<InputState>,
    mut selection: ResMut<SelectionManager>,
    mut query: Query<(Entity, &crate::components::Position, &mut Selectable)>,
) {
    if input.is_mouse_button_just_pressed(MouseButton::Left) {
        let click_pos = input.mouse_world_position;

        if !input.is_key_pressed(KeyCode::Shift) {
            selection.clear();
            for (_, _, mut selectable) in query.iter_mut() {
                selectable.selected = false;
            }
        }

        for (entity, position, mut selectable) in query.iter_mut() {
            let distance = position.0.distance(click_pos);
            if distance <= selectable.radius {
                selectable.selected = true;
                selection.add(entity);
                break;
            }
        }
    }

    if input.is_key_just_pressed(KeyCode::Escape) {
        selection.clear();
        for (_, _, mut selectable) in query.iter_mut() {
            selectable.selected = false;
        }
    }
}

pub fn camera_control_system(
    input: Res<InputState>,
    mut camera: ResMut<crate::resources::Camera2dResource>,
) {
    let move_speed = 500.0;

    if input.is_key_pressed(KeyCode::W) {
        camera.position.y += move_speed * 0.016;
    }
    if input.is_key_pressed(KeyCode::S) {
        camera.position.y -= move_speed * 0.016;
    }
    if input.is_key_pressed(KeyCode::A) {
        camera.position.x -= move_speed * 0.016;
    }
    if input.is_key_pressed(KeyCode::D) {
        camera.position.x += move_speed * 0.016;
    }

    if input.scroll_delta != 0.0 {
        camera.zoom *= 1.0 - input.scroll_delta * 0.1;
        camera.zoom = camera.zoom.clamp(0.1, 10.0);
    }
}
