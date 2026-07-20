use litehouse_management::{
    WorkflowDirectory, WorkflowFileFormat, WorkflowFilesError, WorkflowMutationError,
    WorkflowRepository,
};
use litehouse_rule_json::{HARD_MAX_JSON_BYTES, RuleDocument};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const VALID_MINIMAL: &str = include_str!("../../litehouse-rule-json/fixtures/valid-minimal.json");
const VALID_MAXIMAL: &str = include_str!("../../litehouse-rule-json/fixtures/valid-maximal.json");

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "litehouse-workflows-{label}-{}-{}",
            std::process::id(),
            DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn json_and_yaml_compile_to_identical_rule_semantics() {
    let json_directory = TestDirectory::new("json");
    json_directory.write("lighting/main.json", VALID_MINIMAL);
    let json = WorkflowDirectory::load(json_directory.path()).unwrap();
    let json_workflow = json.workflows().values().next().unwrap();

    let document: RuleDocument = serde_json::from_str(VALID_MINIMAL).unwrap();
    let yaml_source = yaml_serde::to_string(&document).unwrap();
    let yaml_directory = TestDirectory::new("yaml");
    yaml_directory.write("lighting/main.yaml", yaml_source);
    let yaml = WorkflowDirectory::load(yaml_directory.path()).unwrap();
    let yaml_workflow = yaml.workflows().values().next().unwrap();

    assert_eq!(json_workflow.format(), WorkflowFileFormat::Json);
    assert_eq!(yaml_workflow.format(), WorkflowFileFormat::Yaml);
    assert_eq!(
        json_workflow.compiled().semantic_sha256(),
        yaml_workflow.compiled().semantic_sha256()
    );
    assert_eq!(
        json_workflow.compiled().rule_id(),
        yaml_workflow.compiled().rule_id()
    );
    assert!(json_workflow.compiled().enabled());
    assert!(yaml_workflow.compiled().enabled());
}

#[test]
fn maximal_rule_remains_within_the_strict_yaml_event_budget() {
    let document: RuleDocument = serde_json::from_str(VALID_MAXIMAL).unwrap();
    let directory = TestDirectory::new("yaml-maximal");
    directory.write("maximal.yaml", yaml_serde::to_string(&document).unwrap());
    let loaded = WorkflowDirectory::load(directory.path()).unwrap();
    assert_eq!(loaded.workflows().len(), 1);
}

#[test]
fn workflow_discovery_is_recursive_and_directory_digest_is_creation_order_independent() {
    let first = TestDirectory::new("ordered-first");
    first.write("z/second.json", VALID_MINIMAL);
    let mut second_document: serde_json::Value = serde_json::from_str(VALID_MINIMAL).unwrap();
    second_document["rule_id"] = serde_json::json!("11111111111111111111111111111111");
    first.write(
        "a/first.json",
        serde_json::to_vec_pretty(&second_document).unwrap(),
    );
    first.write("README.md", b"ignored");

    let second = TestDirectory::new("ordered-second");
    second.write(
        "a/first.json",
        serde_json::to_vec_pretty(&second_document).unwrap(),
    );
    second.write("README.md", b"ignored");
    second.write("z/second.json", VALID_MINIMAL);

    let first = WorkflowDirectory::load(first.path()).unwrap();
    let second = WorkflowDirectory::load(second.path()).unwrap();
    assert_eq!(first.authored_sha256(), second.authored_sha256());
    let mut paths: Vec<_> = first
        .workflows()
        .values()
        .map(|workflow| workflow.relative_path())
        .collect();
    paths.sort_unstable();
    assert_eq!(paths, vec!["a/first.json", "z/second.json"]);
}

#[test]
fn duplicate_rule_ids_fail_the_entire_directory() {
    let directory = TestDirectory::new("duplicate");
    directory.write("a.json", VALID_MINIMAL);
    directory.write("nested/b.yaml", yaml_for_valid_document());

    assert!(matches!(
        WorkflowDirectory::load(directory.path()),
        Err(WorkflowFilesError::DuplicateRuleId { .. })
    ));
}

#[test]
fn enabled_is_required_authored_state() {
    let mut document: serde_json::Value = serde_json::from_str(VALID_MINIMAL).unwrap();
    document.as_object_mut().unwrap().remove("enabled");
    let directory = TestDirectory::new("missing-enabled");
    directory.write("rule.json", serde_json::to_vec_pretty(&document).unwrap());
    assert!(matches!(
        WorkflowDirectory::load(directory.path()),
        Err(WorkflowFilesError::InvalidRule { .. })
    ));
}

#[test]
fn disabled_is_authored_but_does_not_change_semantic_identity() {
    let enabled_directory = TestDirectory::new("enabled");
    enabled_directory.write("rule.json", VALID_MINIMAL);
    let enabled = WorkflowDirectory::load(enabled_directory.path()).unwrap();

    let mut disabled_document: serde_json::Value = serde_json::from_str(VALID_MINIMAL).unwrap();
    disabled_document["enabled"] = serde_json::json!(false);
    let disabled_directory = TestDirectory::new("disabled");
    disabled_directory.write(
        "rule.json",
        serde_json::to_vec_pretty(&disabled_document).unwrap(),
    );
    let disabled = WorkflowDirectory::load(disabled_directory.path()).unwrap();

    let enabled_rule = enabled.workflows().values().next().unwrap().compiled();
    let disabled_rule = disabled.workflows().values().next().unwrap().compiled();
    assert!(enabled_rule.enabled());
    assert!(!disabled_rule.enabled());
    assert_eq!(
        enabled_rule.semantic_sha256(),
        disabled_rule.semantic_sha256()
    );
}

#[test]
fn yaml_duplicate_keys_tags_non_string_keys_and_floats_fail_closed() {
    let duplicate = TestDirectory::new("yaml-duplicate");
    duplicate.write("rule.yaml", b"rule_id: a\nrule_id: b\n");
    assert!(matches!(
        WorkflowDirectory::load(duplicate.path()),
        Err(WorkflowFilesError::InvalidYaml { .. })
    ));

    for (label, source) in [
        ("tag", "!custom\nrule_id: value\n"),
        ("core-tag", "revision: !!int 1\n"),
        ("anchor", "rule_id: &id value\ncopy: *id\n"),
        ("key", "? [complex, key]\n: value\n"),
        ("float", "revision: 1.5\n"),
    ] {
        let directory = TestDirectory::new(label);
        directory.write("rule.yaml", source);
        assert!(matches!(
            WorkflowDirectory::load(directory.path()),
            Err(WorkflowFilesError::UnsupportedYaml { .. })
        ));
    }
}

#[test]
fn repository_reload_is_all_or_nothing() {
    let directory = TestDirectory::new("repository-reload");
    directory.write("rule.json", VALID_MINIMAL);
    let repository = WorkflowRepository::open(directory.path()).unwrap();
    let original = repository.snapshot().unwrap();
    fs::write(directory.path().join("rule.json"), b"invalid").unwrap();

    assert!(matches!(
        repository.reload(),
        Err(WorkflowMutationError::Files(_))
    ));
    assert_eq!(
        repository.snapshot().unwrap().authored_sha256(),
        original.authored_sha256()
    );
}

#[test]
fn repository_put_uses_content_digests_and_swaps_the_complete_snapshot() {
    let directory = TestDirectory::new("repository-put");
    directory.write("rule.json", VALID_MINIMAL);
    fs::create_dir_all(directory.path().join("nested")).unwrap();
    let repository = WorkflowRepository::open(directory.path()).unwrap();
    let original = repository.snapshot().unwrap();
    let original_workflow = original.workflows().values().next().unwrap();
    let original_digest = original_workflow.source_sha256();

    let mut updated: serde_json::Value = serde_json::from_str(VALID_MINIMAL).unwrap();
    updated["revision"] = serde_json::json!(2);
    let updated_bytes = serde_json::to_vec_pretty(&updated).unwrap();
    let updated_snapshot = repository
        .put("rule.json", Some(original_digest), &updated_bytes)
        .unwrap();
    assert_eq!(
        updated_snapshot
            .workflows()
            .values()
            .next()
            .unwrap()
            .compiled()
            .revision(),
        2
    );
    assert!(matches!(
        repository.put("rule.json", Some(original_digest), &updated_bytes),
        Err(WorkflowMutationError::Conflict)
    ));

    updated["rule_id"] = serde_json::json!("11111111111111111111111111111111");
    let yaml: RuleDocument = serde_json::from_value(updated).unwrap();
    let created = repository
        .put(
            "nested/second.yaml",
            None,
            yaml_serde::to_string(&yaml).unwrap().as_bytes(),
        )
        .unwrap();
    assert_eq!(created.workflows().len(), 2);
}

#[test]
fn repository_rejects_a_create_that_would_exceed_the_complete_tree_limit() {
    let directory = TestDirectory::new("repository-capacity");
    let workflows = directory.path().join("workflows");
    fs::create_dir(&workflows).unwrap();
    fs::write(workflows.join("rule.json"), VALID_MINIMAL).unwrap();
    for index in 0..4_095 {
        fs::write(workflows.join(format!("ignored-{index}.txt")), b"").unwrap();
    }
    let repository =
        WorkflowRepository::open_with_authoring_root(&workflows, directory.path()).unwrap();

    let result = repository.put("second.json", None, VALID_MINIMAL.as_bytes());
    assert!(
        matches!(
            result,
            Err(WorkflowMutationError::Files(
                WorkflowFilesError::TooManyEntries { .. }
            ))
        ),
        "unexpected result: {result:?}"
    );
    assert!(!workflows.join("second.json").exists());
}

#[test]
fn repository_returns_busy_instead_of_waiting_on_another_mutation() {
    let directory = TestDirectory::new("repository-busy");
    directory.write("rule.json", VALID_MINIMAL);
    let repository = WorkflowRepository::open(directory.path()).unwrap();
    let lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(directory.path().join(".litehouse-authoring.lock"))
        .unwrap();
    lock.lock().unwrap();

    assert!(matches!(
        repository.put("second.json", None, VALID_MINIMAL.as_bytes()),
        Err(WorkflowMutationError::Busy)
    ));
}

#[test]
fn repository_rejects_invalid_or_duplicate_writes_before_commit_and_removes_by_digest() {
    let directory = TestDirectory::new("repository-remove");
    directory.write("rule.json", VALID_MINIMAL);
    fs::create_dir_all(directory.path().join("nested")).unwrap();
    let repository = WorkflowRepository::open(directory.path()).unwrap();
    for path in ["../escape.json", "nested//ambiguous.json", "/absolute.json"] {
        assert!(matches!(
            repository.put(path, None, VALID_MINIMAL.as_bytes()),
            Err(WorkflowMutationError::InvalidPath { .. })
        ));
    }
    assert!(
        repository
            .put("nested/invalid.json", None, b"invalid")
            .is_err()
    );
    assert!(!directory.path().join("nested/invalid.json").exists());
    assert!(matches!(
        repository.put("nested/duplicate.json", None, VALID_MINIMAL.as_bytes()),
        Err(WorkflowMutationError::DuplicateRuleId { .. })
    ));
    assert!(!directory.path().join("nested/duplicate.json").exists());

    let snapshot = repository.snapshot().unwrap();
    let workflow = snapshot.workflows().values().next().unwrap();
    assert!(matches!(
        repository.remove("rule.json", [0; 32]),
        Err(WorkflowMutationError::Conflict)
    ));
    let empty = repository
        .remove("rule.json", workflow.source_sha256())
        .unwrap();
    assert!(empty.workflows().is_empty());
    assert!(!directory.path().join("rule.json").exists());
}

#[test]
fn oversized_workflow_is_rejected_from_the_bounded_reader() {
    let directory = TestDirectory::new("oversized");
    directory.write("large.yaml", vec![b' '; HARD_MAX_JSON_BYTES + 1]);
    assert!(matches!(
        WorkflowDirectory::load(directory.path()),
        Err(WorkflowFilesError::FileTooLarge { .. })
    ));
}

#[cfg(unix)]
#[test]
fn workflow_symlinks_fail_closed_even_when_the_target_is_inside_the_root() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new("symlink");
    directory.write("real.json", VALID_MINIMAL);
    symlink("real.json", directory.path().join("alias.json")).unwrap();
    assert!(matches!(
        WorkflowDirectory::load(directory.path()),
        Err(WorkflowFilesError::Symlink { .. })
    ));
}

#[cfg(unix)]
#[test]
fn workflow_root_symlinks_fail_closed() {
    use std::os::unix::fs::symlink;

    let target = TestDirectory::new("root-target");
    target.write("rule.json", VALID_MINIMAL);
    let parent = TestDirectory::new("root-link");
    let alias = parent.path().join("alias");
    symlink(target.path(), &alias).unwrap();
    assert!(matches!(
        WorkflowDirectory::load(alias),
        Err(WorkflowFilesError::Symlink { .. })
    ));
}

fn yaml_for_valid_document() -> String {
    let document: RuleDocument = serde_json::from_str(VALID_MINIMAL).unwrap();
    yaml_serde::to_string(&document).unwrap()
}
