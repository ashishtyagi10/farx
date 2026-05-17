//! Heuristic: should a file be opened in the built-in editor (text) or
//! handed to the system default application (binary/media)?

use std::path::Path;

/// Determine if a file should be opened in the built-in editor (text)
/// or with the system default application (binary/media).
pub(super) fn is_text_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some(
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "c" | "h" | "cpp" | "cc" | "hpp"
            | "java" | "kt" | "swift" | "rb" | "pl" | "pm" | "lua" | "php" | "sh" | "bash" | "zsh"
            | "fish" | "ps1" | "bat" | "cmd" | "html" | "htm" | "css" | "scss" | "less" | "sass"
            | "xml" | "svg" | "json" | "jsonc" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf"
            | "env" | "properties" | "md" | "markdown" | "txt" | "text" | "log" | "csv" | "tsv"
            | "sql" | "graphql" | "gql" | "dockerfile" | "makefile" | "cmake" | "gitignore"
            | "gitattributes" | "editorconfig" | "lock" | "sum" | "r" | "R" | "jl" | "ex" | "exs"
            | "erl" | "hrl" | "elm" | "zig" | "nim" | "v" | "d" | "pas" | "pp" | "tf" | "hcl"
            | "nix" | "dhall" | "proto" | "thrift" | "avsc" | "vue" | "svelte" | "astro",
        ) => true,

        Some(
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
            | "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "tif" | "webp" | "heic"
            | "heif" | "raw" | "cr2" | "nef" | "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma"
            | "m4a" | "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "zip"
            | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "lz" | "dmg" | "iso" | "img"
            | "pkg" | "deb" | "rpm" | "msi" | "exe" | "app" | "so" | "dylib" | "dll" | "a" | "lib"
            | "o" | "obj" | "class" | "jar" | "war" | "pyc" | "pyo" | "wasm" | "ttf" | "otf"
            | "woff" | "woff2" | "eot" | "db" | "sqlite" | "sqlite3" | "psd" | "ai" | "sketch"
            | "fig" | "xd",
        ) => false,

        None => {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            matches!(
                name.to_lowercase().as_str(),
                "makefile"
                    | "dockerfile"
                    | "vagrantfile"
                    | "gemfile"
                    | "rakefile"
                    | "procfile"
                    | "brewfile"
                    | "justfile"
                    | "taskfile"
                    | ".gitignore"
                    | ".gitattributes"
                    | ".editorconfig"
                    | ".env"
                    | ".bashrc"
                    | ".zshrc"
                    | ".profile"
                    | ".vimrc"
                    | "license"
                    | "readme"
                    | "changelog"
                    | "authors"
                    | "todo"
            ) || {
                std::fs::read(path)
                    .map(|bytes| {
                        let check = &bytes[..bytes.len().min(512)];
                        !check.contains(&0)
                    })
                    .unwrap_or(false)
            }
        }

        Some(_) => std::fs::read(path)
            .map(|bytes| {
                let check = &bytes[..bytes.len().min(512)];
                !check.contains(&0)
            })
            .unwrap_or(false),
    }
}
