use std::path::{Path, PathBuf};

pub(crate) const SCREENSHOT_DIR: &str = "screenshots";

pub(crate) fn next_screenshot_file(
    directory: &Path,
    stamp: &str,
    exists: impl Fn(&Path) -> bool,
) -> PathBuf {
    let mut count = 1u32;
    loop {
        let name = if count == 1 {
            format!("{stamp}.png")
        } else {
            format!("{stamp}_{count}.png")
        };
        let candidate = directory.join(name);
        if !exists(&candidate) {
            return candidate;
        }
        count = match count.checked_add(1) {
            Some(next) => next,
            None => return candidate,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_first_free_screenshot_name_follows_the_vanilla_suffix_sequence() {
        let directory = Path::new("/game/screenshots");
        let taken = HashSet::from([
            directory.join("2026-08-05_11.22.33.png"),
            directory.join("2026-08-05_11.22.33_2.png"),
        ]);
        assert_eq!(
            next_screenshot_file(directory, "2026-08-05_11.22.33", |path| taken
                .contains(path)),
            directory.join("2026-08-05_11.22.33_3.png")
        );
        assert_eq!(
            next_screenshot_file(directory, "2026-08-05_11.22.34", |path| taken
                .contains(path)),
            directory.join("2026-08-05_11.22.34.png")
        );
    }
}
