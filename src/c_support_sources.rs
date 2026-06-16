use std::path::{Path, PathBuf};

pub fn companion_sources(input: &Path, source: &str) -> Vec<PathBuf> {
    let Some(root) = input.parent() else {
        return Vec::new();
    };
    let mut paths = Vec::new();

    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    if matches!(stem, "cp" | "mv")
        || contains_call(source, "enmasse")
        || contains_call(source, "cp")
        || source.contains("cp_iflag =")
        || source.contains("cp_status =")
    {
        for rel in [
            "libutil/cp.c",
            "libutil/enmasse.c",
            "libutil/fnck.c",
            "libutil/confirm.c",
            "libutil/concat.c",
            "libutil/writeall.c",
        ] {
            push_if_exists(&mut paths, root.join(rel));
        }
    }

    if contains_call(source, "concat") {
        for rel in ["libutil/concat.c", "libutil/writeall.c"] {
            push_if_exists(&mut paths, root.join(rel));
        }
    } else if contains_call(source, "writeall") {
        push_if_exists(&mut paths, root.join("libutil/writeall.c"));
    }

    if contains_call(source, "getlines") {
        push_if_exists(&mut paths, root.join("libutil/getlines.c"));
    }
    if contains_call(source, "linecmp") {
        push_if_exists(&mut paths, root.join("libutil/linecmp.c"));
    }

    paths
}

fn push_if_exists(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.exists() && !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn contains_call(source: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    source.lines().any(|line| {
        let trimmed = line.trim_start();
        if !trimmed.contains(&needle) {
            return false;
        }
        if trimmed.starts_with(&format!("int {name}("))
            || trimmed.starts_with(&format!("void {name}("))
            || trimmed.starts_with(&format!("ssize_t {name}("))
            || trimmed.starts_with(&format!("size_t {name}("))
            || trimmed.starts_with(&format!("char *{name}("))
            || trimmed.starts_with(&format!("void *{name}("))
        {
            return false;
        }
        true
    })
}
