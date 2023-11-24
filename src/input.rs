use winit::keyboard::KeyCode;

pub enum Axis {
    Keys(KeyCode, KeyCode),
    Native(),
}

pub struct InputBindings {
    forward: (),
    right: (),
    up: (),
}