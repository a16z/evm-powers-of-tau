use ethers_solc::{Project, ProjectPathsConfig};
use std::path::PathBuf;

fn main() {
    // Build all solidity files under /contracts
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("contracts");
    let paths = ProjectPathsConfig::builder()
        .root(&root)
        .sources(&root)
        .build()
        .unwrap();
    let mut project = Project::builder().paths(paths).ephemeral().build().unwrap();
    project.solc_config.settings.optimizer.enable();
    project.solc_config.settings.optimizer.runs(200);
    project.compile().unwrap();
    let compiler_output = project.compile().unwrap();
    assert!(
        !compiler_output.has_compiler_errors(),
        "{}",
        compiler_output.to_string()
    );
}
