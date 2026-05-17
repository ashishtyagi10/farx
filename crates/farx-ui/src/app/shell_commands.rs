//! Classifier: does a command-line entry look like a shell command (vs.
//! a natural-language query for the AI bar)?

/// Heuristically decide whether `input` looks like something the system
/// shell should execute (otherwise it's treated as an AI query).
pub(super) fn looks_like_shell_command(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }

    let first_word = trimmed.split_whitespace().next().unwrap_or("");

    if first_word.starts_with('/') || first_word.starts_with("./") || first_word.starts_with("~/") {
        return true;
    }

    if trimmed.contains('|')
        || trimmed.contains('>')
        || trimmed.contains('<')
        || trimmed.contains("&&")
        || trimmed.contains("||")
        || trimmed.contains(';')
    {
        return true;
    }

    if SHELL_COMMANDS.contains(&first_word) {
        return true;
    }

    if first_word.contains('=') && !first_word.starts_with('=') {
        return true;
    }

    if first_word.contains('.')
        && (first_word.ends_with(".sh")
            || first_word.ends_with(".py")
            || first_word.ends_with(".rb")
            || first_word.ends_with(".js")
            || first_word.ends_with(".pl"))
    {
        return true;
    }

    false
}

const SHELL_COMMANDS: &[&str] = &[
    "ls",
    "cd",
    "cp",
    "mv",
    "rm",
    "mkdir",
    "rmdir",
    "cat",
    "head",
    "tail",
    "grep",
    "find",
    "sed",
    "awk",
    "sort",
    "uniq",
    "wc",
    "echo",
    "printf",
    "touch",
    "chmod",
    "chown",
    "chgrp",
    "ln",
    "pwd",
    "env",
    "export",
    "which",
    "whereis",
    "whoami",
    "date",
    "cal",
    "df",
    "du",
    "free",
    "top",
    "ps",
    "kill",
    "tar",
    "zip",
    "unzip",
    "gzip",
    "gunzip",
    "curl",
    "wget",
    "ssh",
    "scp",
    "rsync",
    "git",
    "docker",
    "make",
    "npm",
    "yarn",
    "pnpm",
    "cargo",
    "rustc",
    "python",
    "python3",
    "pip",
    "node",
    "ruby",
    "go",
    "java",
    "javac",
    "gcc",
    "g++",
    "clang",
    "brew",
    "apt",
    "yum",
    "dnf",
    "pacman",
    "snap",
    "flatpak",
    "systemctl",
    "journalctl",
    "sudo",
    "su",
    "man",
    "less",
    "more",
    "vi",
    "vim",
    "nano",
    "emacs",
    "code",
    "open",
    "xdg-open",
    "clear",
    "reset",
    "history",
    "alias",
    "unalias",
    "set",
    "unset",
    "test",
    "true",
    "false",
    "yes",
    "no",
    "tee",
    "xargs",
    "diff",
    "patch",
    "file",
    "stat",
    "md5",
    "sha256sum",
    "base64",
];
