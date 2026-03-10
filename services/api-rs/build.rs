use std::path::Path;
use std::process::Command;

fn main() {
    let ast_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("governance")
        .join("ast");

    // Re-run build if any AST source file changes
    println!(
        "cargo:rerun-if-changed={}",
        ast_dir.join("meta.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ast_dir.join("rules.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ast_dir.join("structured-data.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ast_dir.join("documents").display()
    );

    let assemble_script = ast_dir.join("assemble.py");
    if !assemble_script.exists() {
        // If the assembler doesn't exist, assume governance-ast.json is already current
        return;
    }

    let status = Command::new("python3")
        .arg(&assemble_script)
        .arg(&ast_dir)
        .status()
        .expect("failed to run governance AST assembler (python3 required)");

    if !status.success() {
        panic!("governance AST assembly failed");
    }
}
