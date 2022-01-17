use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use slotmap::DefaultKey;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::unix::EventLoopExtUnix;
use winit::window::{Window, WindowBuilder};

use crate::{HEIGHT, RectObject, State, WIDTH};
use crate::fps_counter::default_counter;
use crate::input_state::InputState;
use crate::text::{TextInfo, TextObject, TextObjectHandle};

struct Cursor(DefaultKey);

impl Cursor {
    const COLORONE: [u8; 4] = [255, 255, 255, 255];
    const COLORTWO: [u8; 4] = [60, 60, 60, 255];
    fn update(&self, s: &mut State, text: &TextObjectHandle, time: u8) {
        let TextInfo { max: br, .. } = text.get_stats(&s.tp).unwrap();

        // Set position to the bottom right of the text passage
        let rect = s.rp.rects.get_mut(self.0).unwrap();
        rect.x = br.0 as u32 / 64;
        rect.y = br.1 as u32 / 64;

        if time > u8::MAX / 2 {
            rect.color = Self::COLORONE;
        } else {
            rect.color = Self::COLORTWO;
        }
    }
}

pub struct Layout {
    text_key: Vec<TextObjectHandle>,
    cursor: Vec<Cursor>,
    time: u8,
}

impl Layout {
    fn new(v: Vec<TextObjectHandle>, state: &mut State) -> Self {
        let mut cursors = Vec::new();
        for _ in &v {
            cursors.push(Cursor(state.rp.add_rect(RectObject {
                x: 0,
                y: 0,
                w: 10,
                h: state.tp.fontatl.font_height() / 64,
                color: [100, 100, 100, 255],
            })));
        }
        Self {
            text_key: v,
            cursor: cursors,
            time: 0,
        }
    }

    fn update(&mut self, s: &mut State) {
        self.time = self.time.wrapping_add(5);
        for (text, cursor) in self.text_key.iter().zip(self.cursor.iter()) {
            cursor.update(s, text, self.time);
        }
    }
}


pub struct Terminal {
    s: State,
    cursor: Layout,
    input_state: InputState,
    event_loop: Option<EventLoop<()>>,
    window: Window,
    latency: Option<Instant>,
}

unsafe impl Send for Terminal {}

pub struct TerminalWindow<'a>(usize, &'a mut Terminal);

impl<'a> Write for TerminalWindow<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.1.send_text(std::str::from_utf8(buf).unwrap(), self.0);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> TerminalWindow<'a> {
    fn clear(&mut self) {
        self.1.set_text("", self.0);
    }
}

impl Terminal {
    pub fn new() -> Self {
        let event_loop = EventLoop::new_any_thread();
        let window = WindowBuilder::new().with_inner_size(winit::dpi::PhysicalSize {
            width: WIDTH,
            height: HEIGHT,
        }).build(&event_loop).unwrap();

        let mut state = State::new(&window);
        let text_key_l = state.tp.add_text(TextObject {
            render_str: "hello world".to_string(),
            top_left: (10, HEIGHT as i32 - 10),
            max_width: WIDTH / 2 - 10,
            dirty: false,
        });
        let text_key_r = state.tp.add_text(TextObject {
            render_str: "hello world".to_string(),
            top_left: (20 + WIDTH as i32 / 2, HEIGHT as i32 - 10),
            max_width: WIDTH / 2 - 10,
            dirty: false,
        });
        let cursor = Layout::new(vec![text_key_l, text_key_r], &mut state);
        Terminal {
            s: state,
            cursor,
            window,
            event_loop: Some(event_loop),
            input_state: Default::default(),
            latency: Default::default(),
        }
    }

    pub fn send_text(&mut self, t: &str, location: usize) {
        self.cursor.text_key[location].append_str(&mut self.s.tp, t);
    }
    pub fn set_text(&mut self, t: &str, location: usize) {
        self.cursor.text_key[location].update_str(&mut self.s.tp, t.to_string());
    }
    pub fn nth_window(&mut self, n: usize) -> TerminalWindow<'_> {
        TerminalWindow(n, self)
    }

    pub fn update(&mut self) {
        if !self.input_state.key_buffer.is_empty() {
            let as_str = String::from_iter(self.input_state.key_buffer.iter());
            self.cursor.text_key[0].append_str(&mut self.s.tp, &as_str);
            self.cursor.text_key[1].append_str(&mut self.s.tp, "fdsa fdlks;a j;f jsalknsa vc.mfda");
            self.input_state.key_buffer.clear();
        }

        for i in &self.cursor.text_key {
            if let Some(stats) = i.get_stats(&self.s.tp).cloned() {
                let fh = self.s.tp.fontatl.font_height() as i32;
                if stats.max.1 < fh {
                    i.add_offset(&mut self.s.tp, (0, (fh - stats.max.1) / 64));
                }
            }
        }

        if self.input_state.scroll != (0, 0) {
            let scroll_pos = self.input_state.mouse_pos;
            if scroll_pos.0 < WIDTH as i32 / 2 {
                self.cursor.text_key[0].add_offset(&mut self.s.tp, self.input_state.scroll);
            } else {
                self.cursor.text_key[1].add_offset(&mut self.s.tp, self.input_state.scroll);
            }
            self.input_state.scroll = (0, 0);
        }
        self.s.update();
        self.cursor.update(&mut self.s);
    }

    pub fn render(&mut self) {
        self.s.render().unwrap();
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.input_state.process_input(event);
        if matches!(event, WindowEvent::KeyboardInput {..}) {
            self.latency = Some(Instant::now());
        }
        true
    }

    pub fn run(se: Arc<Mutex<Option<Self>>>) {
        // let mut _l = se.lock();
        // let se = _l.as_mut().unwrap();
        let ev = se.lock().as_mut().unwrap().as_mut().unwrap().event_loop.take().unwrap();
        ev.run(move |event, _, control_flow| {
            let mut lock = se.lock();
            let se = lock.as_mut().unwrap().as_mut().unwrap();
            *control_flow = ControlFlow::Wait;
            log::debug!("Event {:?}", event);
            let mut should_draw = false;
            match event {
                Event::WindowEvent {
                    event,
                    window_id,
                } if window_id == se.window.id() => {
                    se.input(&event);
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { .. } => should_draw = true,
                        _ => {}
                    }
                }
                Event::RedrawRequested(_) => {
                    should_draw = true;
                }
                Event::RedrawEventsCleared => {
                    se.window.request_redraw();
                }
                _ => {}
            }
            if should_draw {
                let time = Instant::now();
                se.update();
                se.render();
                // println!("Render time {}", time.elapsed().as_millis());

                if let Some(time) = se.latency.take() {
                    log::debug!("Key Latency {}", time.elapsed().as_millis());
                }
                default_counter().frame();
                default_counter().report();
            }
        });
    }
}

pub fn test() {
    let t = Arc::new(Mutex::new(None));
    let t1 = t.clone();
    thread::spawn(move || {
        *t1.lock().unwrap() = Some(Terminal::new());
        Terminal::run(t1);
    });
    while t.lock().unwrap().is_none() {
        std::thread::sleep(Duration::from_millis(10));
    }

    let rand_str = &include_str!("../rand");
    loop {
        let mut lock = t.lock().unwrap();
        let t1 = lock.as_mut().unwrap();
        writeln!(t1.nth_window(0), "{}", &rand_str[000..050]).unwrap();
        writeln!(t1.nth_window(1), "{}", &rand_str[100..150]).unwrap();
        writeln!(t1.nth_window(1), "{}", &rand_str[150..200]).unwrap();
        writeln!(t1.nth_window(0), "{}", &rand_str[200..250]).unwrap();
        std::mem::drop(lock);
        std::thread::sleep(Duration::from_millis(1));
    }
    // writeln!(term.nth_window(1), "helo").unwrap();
    loop {}
}