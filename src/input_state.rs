use winit::event::{ElementState, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent};

#[derive(Default)]
pub struct InputState {
    pub mouse_pos: (i32, i32),
    pub modifiers: ModifiersState,
    pub key_buffer: Vec<char>,
    pub scroll: (i32, i32),
}

impl InputState {
    pub fn process_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseWheel { delta, phase, .. } => {
                println!("Scrolling {:?}", delta);
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (*x as i32, *y as i32),
                    MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition { x, y }) => ((*x) as i32, (*y) as i32)
                };
                self.scroll = delta;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x as i32, position.y as i32);
            }
            WindowEvent::ModifiersChanged(state) => {
                self.modifiers = *state;
            }

            WindowEvent::ReceivedCharacter(char) => {
                let char = if self.modifiers.shift() {
                    char.to_ascii_uppercase()
                } else {
                    char.to_ascii_lowercase()
                };
                self.key_buffer.push(char);
            }
            _ => {}
        }
    }
}