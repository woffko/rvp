//! The processors' time for the log's seconds of playback (Linux, from
//! `/proc`): the time all processors were busy, their whole time, and the
//! time RVP's own process took, all in the kernel's clock ticks.

/// All processors' busy and whole time, and this process's, so far.
#[must_use]
pub fn times() -> Option<[u64; 3]> {
    parse(
        &std::fs::read_to_string("/proc/stat").ok()?,
        &std::fs::read_to_string("/proc/self/stat").ok()?,
    )
}

/// The times from `/proc/stat` and `/proc/self/stat`.
fn parse(stat: &str, own: &str) -> Option<[u64; 3]> {
    // cpu  user nice system idle iowait irq softirq steal guest guest_nice;
    // the guests' time is in user and nice already.
    let fields: Vec<u64> = stat
        .lines()
        .next()?
        .strip_prefix("cpu ")?
        .split_whitespace()
        .take(8)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let total: u64 = fields.iter().sum();
    let waiting = fields.get(3)? + fields.get(4).copied().unwrap_or(0);
    // The fields after the command's name, which may hold spaces and
    // parentheses: state is the first, utime the 12th and stime the 13th.
    let after_name = own.rsplit_once(')')?.1;
    let mut own_fields = after_name.split_whitespace().skip(11);
    let user: u64 = own_fields.next()?.parse().ok()?;
    let system: u64 = own_fields.next()?.parse().ok()?;
    Some([total.saturating_sub(waiting), total, user + system])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_files_give_the_times() {
        let stat = "cpu  100 5 50 800 40 3 2 0 7 0\ncpu0 50 2 25 400 20 1 1 0 0 0\n";
        let own =
            "4242 (rvp (main)) S 1 4242 4242 0 -1 4194560 900 0 0 0 31 9 0 0 20 0 30 0 100 0 0\n";
        assert_eq!(parse(stat, own), Some([160, 1000, 40]));
        assert_eq!(parse("intr 1 2 3\n", own), None);
        assert!(times().is_some_and(|[busy, total, _]| busy <= total));
    }
}
