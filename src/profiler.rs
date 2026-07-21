use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Stat {
    total: Duration,
    calls: u64,
}

#[derive(Default)]
pub struct Profiler {
    stats: HashMap<&'static str, Stat>,
}

impl Profiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn profile<F, R>(&mut self, label: &'static str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = start.elapsed();

        let stat = self.stats.entry(label).or_default();
        stat.total += elapsed;
        stat.calls += 1;

        result
    }

    pub fn print(&self) {
        println!("\n=== Profile Summary ===");
        println!(
            "{:<30} {:>8} {:>12} {:>12}",
            "Label", "Calls", "Total", "Average"
        );

        let mut items: Vec<_> = self.stats.iter().collect();
        items.sort_by_key(|(_, s)| std::cmp::Reverse(s.total));

        for (label, stat) in items {
            let avg = if stat.calls == 0 {
                Duration::ZERO
            } else {
                stat.total.div_f64(stat.calls as f64)
            };

            println!(
                "{:<30} {:>8} {:>12?} {:>12?}",
                label, stat.calls, stat.total, avg
            );
        }
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        self.print();
    }
}
