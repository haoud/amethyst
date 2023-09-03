#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum WindowEvent {
    LoopDestroyed,
    MainLoop,
    Exit,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Continue,
    Exit,
}
