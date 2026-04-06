pub mod archive;
pub mod duplicates;
pub mod local;
pub mod ops;

pub use archive::{compress_to_zip, extract_archive, is_archive, list_archive, ArchiveEntry};
pub use duplicates::{find_duplicates, DuplicateGroup};
pub use local::read_directory;
pub use ops::{copy_entry, create_directory, delete_entry, move_entry, rename_entry};
