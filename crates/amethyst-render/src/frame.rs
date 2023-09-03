use std::time::Instant;

/// A simple framerate counter.
pub struct Framerate {
    start_time: Instant,
    frames: u64,
}

impl Framerate {
    // Create a new framerate counter. The counter is initialized (i.e started) with
    // the current time.
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            frames: 0,
        }
    }

    /// Update the framerate counter. It simply increments the frame
    /// counter by one.
    pub fn update(&mut self) {
        self.frames += 1;
    }

    /// Return the current framerate.
    pub fn fps(&self) -> u64 {
        let elapsed = Instant::now() - self.start_time;
        let seconds = elapsed.as_secs_f64();
        (self.frames as f64 / seconds) as u64
    }

    /// Return the elapsed time since the start of the counter.
    pub fn elapsed(&self) -> f64 {
        let elapsed = Instant::now() - self.start_time;
        elapsed.as_secs_f64()
    }

    /// Return the number of frames since the start of the counter.
    pub fn counter(&self) -> u64 {
        self.frames
    }

    /// Reset the framerate counter. This reset the frame count and set the start
    /// time to the current time.
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.frames = 0;
    }
}
