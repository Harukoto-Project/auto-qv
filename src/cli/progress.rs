#[derive(Debug, Default)]
pub struct ProgressPrinter {
    last_percent: Option<u64>,
}

impl ProgressPrinter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, done: usize, total: usize) {
        let value = percent(done, total);
        if self.last_percent != Some(value) {
            self.last_percent = Some(value);
            eprint!("\rWriting... {value:>3}%");
        }
    }

    pub fn finish(&self) {
        if self.last_percent.is_some() {
            eprintln!();
        }
    }
}

fn percent(done: usize, total: usize) -> u64 {
    if total == 0 {
        return 100;
    }
    (done.min(total) as u64 * 100) / total as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_whole_percentages() {
        assert_eq!(percent(0, 4), 0);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(4, 4), 100);
    }

    #[test]
    fn empty_work_counts_as_complete() {
        assert_eq!(percent(0, 0), 100);
    }

    #[test]
    fn never_exceeds_one_hundred_percent() {
        assert_eq!(percent(9, 3), 100);
    }
}
