pub mod archive;
pub mod duplicates;
pub mod local;
pub mod ops;

pub use archive::{compress_to_zip, extract_archive, is_archive, list_archive, ArchiveEntry};
pub use duplicates::{find_duplicates, DuplicateGroup};
pub use local::read_directory;
pub use ops::{
    copy_entries_with_progress, copy_entry, create_directory, create_symlink, delete_entry,
    move_entries_with_progress, move_entry, rename_entry, FileProgress,
};
