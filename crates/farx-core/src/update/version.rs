//! Print the current crate version.

use self_update::cargo_crate_version;

/// Print the current version.
pub fn print_version() {
    println!("farx {}", cargo_crate_version!());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_version_runs() {
        // Smoke test: must not panic and the crate version is non-empty.
        print_version();
        assert!(!cargo_crate_version!().is_empty());
    }
}
