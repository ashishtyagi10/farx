/// Result of the chmod dialog interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum ChmodAction {
    None,
    /// Apply the new mode.
    Apply(u32),
    Cancel,
}

/// Permission bit masks for the 9 standard Unix permission bits, in the order
/// owner-r, owner-w, owner-x, group-r, group-w, group-x, other-r, other-w, other-x.
pub(super) const PERM_MASKS: [u32; 9] = [
    0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
];

/// State for the chmod / file permissions dialog.
pub struct ChmodDialogState {
    /// The file path being modified.
    pub file_path: std::path::PathBuf,
    /// Permission bits as a 9-element array: [owner_r, owner_w, owner_x, group_r, group_w, group_x, other_r, other_w, other_x].
    pub bits: [bool; 9],
    /// Currently focused bit index (0..8).
    pub cursor: usize,
}

impl ChmodDialogState {
    /// Create a new chmod dialog from a Unix mode value.
    pub fn new(file_path: std::path::PathBuf, mode: u32) -> Self {
        let mut bits = [false; 9];
        for (i, &mask) in PERM_MASKS.iter().enumerate() {
            bits[i] = mode & mask != 0;
        }
        Self {
            file_path,
            bits,
            cursor: 0,
        }
    }

    /// Convert the bits array back to a Unix mode value (lower 9 bits).
    pub fn to_mode(&self) -> u32 {
        let mut mode = 0u32;
        for (i, &set) in self.bits.iter().enumerate() {
            if set {
                mode |= PERM_MASKS[i];
            }
        }
        mode
    }
}

/// Format a mode as rwxrwxrwx string.
pub(super) fn format_rwx(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    let chars = ['r', 'w', 'x'];
    for (i, &mask) in PERM_MASKS.iter().enumerate() {
        if mode & mask != 0 {
            s.push(chars[i % 3]);
        } else {
            s.push('-');
        }
    }
    s
}
