mod helpers;

use loom_core::registry::discover_repos;

#[test]
fn test_discover_repos_integration() {
    let env = helpers::TempWorkspace::new();
    env.add_repo("dasch-swiss", "dsp-api");
    env.add_repo("dasch-swiss", "dsp-das");
    env.add_repo("other-org", "my-tool");

    let repos = discover_repos(&env.config.registry.scan_roots, Some(&env.workspace_root));

    assert_eq!(repos.len(), 3);

    let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"dsp-api"));
    assert!(names.contains(&"dsp-das"));
    assert!(names.contains(&"my-tool"));
}

#[test]
fn test_workspace_detection_integration() {
    let env = helpers::TempWorkspace::new();

    // Create a workspace with a manifest
    let ws_name = "test-feature";
    let ws_path = env.workspace_root.join(ws_name);
    std::fs::create_dir_all(&ws_path).unwrap();

    let manifest = loom_core::manifest::WorkspaceManifest {
        name: ws_name.to_string(),
        branch: None,
        created: chrono::Utc::now(),
        base_branch: Some("main".to_string()),
        preset: None,
        repos: vec![],
    };
    loom_core::manifest::write_manifest(
        &ws_path.join(loom_core::workspace::MANIFEST_FILENAME),
        &manifest,
    )
    .unwrap();

    // Detect from workspace root
    let result = loom_core::workspace::detect_workspace(&ws_path).unwrap();
    assert!(result.is_some());
    let (path, loaded) = result.unwrap();
    assert_eq!(path, ws_path);
    assert_eq!(loaded.name, ws_name);

    // Resolve by name
    let (_, resolved) =
        loom_core::workspace::resolve_workspace(Some(ws_name), &env.workspace_root, &env.config)
            .unwrap();
    assert_eq!(resolved.name, ws_name);
}
