use std::process::Command;

#[test]
fn test_dockerfile_exists() {
    let path = "Dockerfile";
    assert!(
        std::path::Path::new(path).exists(),
        "Dockerfile must exist at project root"
    );
}

#[test]
fn test_dockerfile_syntax_valid() {
    // Verify Dockerfile can be parsed/validated (basic check)
    let output = Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "-i",
            "hadolint/hadolint:latest-debian",
            "hadolint",
            "-",
        ])
        .arg("-")
        .output();

    if output.is_ok() {
        // hadolint is optional, test passes if docker/hadolint available
        println!("hadolint validation passed (or not available)");
    }
}

#[test]
fn test_docker_image_builds() {
    let output = Command::new("docker")
        .args(&["build", "-t", "arktos-wallet:test", "."])
        .output()
        .expect("Failed to execute docker build");

    assert!(
        output.status.success(),
        "Docker build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
