use std::io::IsTerminal;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

pub trait Progress {
    fn begin(&mut self, label: &str);
    fn update(&mut self, received: u64, total: Option<u64>);
    fn finish(&mut self);
    fn notice(&mut self, message: &str);
}

pub struct StderrProgress {
    interactive: bool,
    bar: Option<ProgressBar>,
}

impl StderrProgress {
    pub fn new() -> Self {
        Self {
            interactive: std::io::stderr().is_terminal(),
            bar: None,
        }
    }
}

impl Progress for StderrProgress {
    fn begin(&mut self, label: &str) {
        if !self.interactive {
            eprintln!("{label}...");
            return;
        }
        let bar = ProgressBar::with_draw_target(Some(1), ProgressDrawTarget::stderr());
        bar.set_style(
            ProgressStyle::with_template("{msg:<32} [{bar:30}] {percent:>3}%")
                .expect("valid progress template")
                .progress_chars("##-"),
        );
        bar.set_message(label.to_string());
        self.bar = Some(bar);
    }

    fn update(&mut self, received: u64, total: Option<u64>) {
        if let Some(bar) = &self.bar {
            bar.set_length(total.unwrap_or(received).max(1));
            bar.set_position(received);
        }
    }

    fn finish(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish();
        }
    }

    fn notice(&mut self, message: &str) {
        eprintln!("{message}");
    }
}
