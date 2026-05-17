use super::types::QuickAction;

pub(super) fn build_actions(
    file_name: &str,
    extension: Option<&str>,
    is_dir: bool,
) -> Vec<QuickAction> {
    let mut actions = Vec::new();

    // Universal action
    actions.push(QuickAction {
        label: "Open with system default".to_string(),
        command: "__open__".to_string(),
    });

    if is_dir {
        push_dir_actions(&mut actions);
    } else {
        push_file_base_actions(&mut actions);
        push_extension_actions(&mut actions, file_name, extension);
        push_generic_file_actions(&mut actions, file_name);
    }

    actions
}

fn push_dir_actions(actions: &mut Vec<QuickAction>) {
    actions.push(QuickAction {
        label: "Open terminal here".to_string(),
        command: "__terminal__".to_string(),
    });
    actions.push(QuickAction {
        label: "Count files recursively".to_string(),
        command: "find . -type f | wc -l".to_string(),
    });
}

fn push_file_base_actions(actions: &mut Vec<QuickAction>) {
    actions.push(QuickAction {
        label: "View in editor".to_string(),
        command: "__edit__".to_string(),
    });
    actions.push(QuickAction {
        label: "View file".to_string(),
        command: "__view__".to_string(),
    });
    actions.push(QuickAction {
        label: "Copy path to clipboard".to_string(),
        command: "__clipboard__".to_string(),
    });
}

fn push_generic_file_actions(actions: &mut Vec<QuickAction>, file_name: &str) {
    actions.push(QuickAction {
        label: "File info (stat)".to_string(),
        command: format!("stat {}", file_name),
    });
    actions.push(QuickAction {
        label: "Checksum (SHA-256)".to_string(),
        command: format!("shasum -a 256 {}", file_name),
    });
}

fn push_extension_actions(
    actions: &mut Vec<QuickAction>,
    file_name: &str,
    extension: Option<&str>,
) {
    match extension {
        Some("rs") => push_rust(actions),
        Some("py") => push_python(actions, file_name),
        Some("js") | Some("ts") | Some("jsx") | Some("tsx") => push_node(actions, file_name),
        Some("sh") | Some("bash") | Some("zsh") => push_shell(actions, file_name),
        Some("go") => push_go(actions, file_name),
        Some("json") => actions.push(QuickAction {
            label: "Pretty print (jq)".to_string(),
            command: format!("jq . {}", file_name),
        }),
        Some("md") | Some("markdown") => actions.push(QuickAction {
            label: "Word count".to_string(),
            command: format!("wc -w {}", file_name),
        }),
        Some("zip") | Some("tar") | Some("gz") | Some("tgz") => push_archive(actions),
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") => {
            actions.push(QuickAction {
                label: "Get image dimensions".to_string(),
                command: format!("file {}", file_name),
            });
        }
        _ => {}
    }
}

fn push_rust(actions: &mut Vec<QuickAction>) {
    actions.push(QuickAction {
        label: "Cargo check".to_string(),
        command: "cargo check".to_string(),
    });
    actions.push(QuickAction {
        label: "Cargo test".to_string(),
        command: "cargo test".to_string(),
    });
    actions.push(QuickAction {
        label: "Cargo run".to_string(),
        command: "cargo run".to_string(),
    });
}

fn push_python(actions: &mut Vec<QuickAction>, file_name: &str) {
    actions.push(QuickAction {
        label: "Run with Python".to_string(),
        command: format!("python3 {}", file_name),
    });
    actions.push(QuickAction {
        label: "Lint (ruff)".to_string(),
        command: format!("ruff check {}", file_name),
    });
}

fn push_node(actions: &mut Vec<QuickAction>, file_name: &str) {
    actions.push(QuickAction {
        label: "Run with Node".to_string(),
        command: format!("node {}", file_name),
    });
    actions.push(QuickAction {
        label: "Lint (eslint)".to_string(),
        command: format!("npx eslint {}", file_name),
    });
}

fn push_shell(actions: &mut Vec<QuickAction>, file_name: &str) {
    actions.push(QuickAction {
        label: "Run script".to_string(),
        command: format!("sh {}", file_name),
    });
    actions.push(QuickAction {
        label: "Make executable".to_string(),
        command: format!("chmod +x {}", file_name),
    });
}

fn push_go(actions: &mut Vec<QuickAction>, file_name: &str) {
    actions.push(QuickAction {
        label: "Go run".to_string(),
        command: format!("go run {}", file_name),
    });
    actions.push(QuickAction {
        label: "Go test".to_string(),
        command: "go test ./...".to_string(),
    });
}

fn push_archive(actions: &mut Vec<QuickAction>) {
    actions.push(QuickAction {
        label: "Extract archive".to_string(),
        command: "__extract__".to_string(),
    });
    actions.push(QuickAction {
        label: "List contents".to_string(),
        command: "__view_archive__".to_string(),
    });
}
