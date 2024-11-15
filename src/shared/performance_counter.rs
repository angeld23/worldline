use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PerformanceCounter {
    times: Vec<Duration>,
    recording_start: Instant,
    last_tick: Instant,
}

impl Default for PerformanceCounter {
    fn default() -> Self {
        Self {
            times: Default::default(),
            recording_start: Instant::now(),
            last_tick: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PerformanceReport {
    pub mean: Duration,
    pub slowest: Duration,
    pub fastest: Duration,
    pub start: Instant,
    pub end: Instant,
}

impl PerformanceCounter {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn tick(&mut self) {
        self.push_time(self.last_tick.elapsed());
        self.last_tick = Instant::now();
    }

    pub fn push_time(&mut self, time: Duration) {
        if self.times.is_empty() {
            self.recording_start = Instant::now() - time;
        }
        self.times.push(time);
    }

    pub fn report(&self) -> Option<PerformanceReport> {
        if self.times.is_empty() {
            return None;
        }

        let mean = self.times.iter().sum::<Duration>() / self.times.len() as u32;
        let (slowest, fastest) = self.times.iter().fold(
            (Duration::ZERO, Duration::MAX),
            |(slowest, fastest), &time| (time.max(slowest), time.min(fastest)),
        );

        Some(PerformanceReport {
            mean,
            slowest,
            fastest,
            start: self.recording_start,
            end: Instant::now(),
        })
    }

    pub fn flush(&mut self) -> Option<PerformanceReport> {
        let report = self.report();
        self.times.clear();
        report
    }
}
