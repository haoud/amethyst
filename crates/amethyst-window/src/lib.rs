use amethyst_core::prelude::Engine;
use prelude::{Status, WindowEvent};

pub mod prelude {
    pub use crate::event::*;
    pub use crate::*;
}

pub mod event;

pub struct Window {
    event_loop: winit::event_loop::EventLoop<()>,
    inner: winit::window::Window,
}

impl Window {
    #[must_use]
    pub fn new(_engine: &mut Engine, info: WindowInfo) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();

        let inner = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(info.width, info.height))
            .with_resizable(info.resizable)
            .with_title(info.title)
            .build(&event_loop)
            .expect("Failed to create a window");

        Self { event_loop, inner }
    }

    pub fn run<F>(self, mut engine: Engine, mut runner: F) -> !
    where
        F: 'static + FnMut(&mut Engine, WindowEvent) -> Status,
    {
        self.event_loop
            .run(move |event, _, control_flow| match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => {
                    let status = runner(&mut engine, WindowEvent::Exit);
                    if status == Status::Exit {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                }

                winit::event::Event::LoopDestroyed => {
                    runner(&mut engine, WindowEvent::LoopDestroyed);
                }

                winit::event::Event::MainEventsCleared => {
                    let status = runner(&mut engine, WindowEvent::MainLoop);
                    if status == Status::Exit {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                }
                _ => (),
            });
    }

    pub fn inner(&self) -> &winit::window::Window {
        &self.inner
    }
}

pub struct WindowInfo<'a> {
    pub title: &'a str,
    pub height: u32,
    pub width: u32,
    pub resizable: bool,
}

impl Default for WindowInfo<'_> {
    fn default() -> Self {
        Self {
            title: "Amethyst window",
            height: 600,
            width: 800,
            resizable: false,
        }
    }
}
