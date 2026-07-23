use cap_fs_ext::{
    DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt, ambient_authority,
};
#[cfg(unix)]
use cap_std::fs::OpenOptionsExt;
use cap_std::fs::{Dir, OpenOptions};
use litehouse_rule_json::{
    CompiledRule, HARD_MAX_COLLECTION_ITEMS, HARD_MAX_JSON_BYTES, HARD_MAX_JSON_DEPTH,
    HARD_MAX_STRING_BYTES, RuleDocument, RuleJsonError, compile_rule_json,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering as AtomicOrdering},
    },
};
use thiserror::Error;
use yaml_rust2::parser::{Event as YamlEvent, Parser as YamlParser};
use yaml_serde::Value as YamlValue;

const MAX_WORKFLOW_FILES: usize = 256;
const MAX_WORKFLOW_DIRECTORIES: usize = 512;
const MAX_WORKFLOW_ENTRIES: usize = 4_096;
const MAX_WORKFLOW_DIRECTORY_DEPTH: usize = 16;
const MAX_WORKFLOW_RELATIVE_PATH_BYTES: usize = 512;
const MAX_YAML_EVENTS: usize = 8_192;
const DIRECTORY_DIGEST_DOMAIN: &[u8] = b"litehouse-workflow-directory-v1\0";
static WORKFLOW_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowFileFormat {
    Json,
    Yaml,
}

#[derive(Debug, Clone)]
pub struct LoadedWorkflow {
    relative_path: String,
    format: WorkflowFileFormat,
    source_sha256: [u8; 32],
    compiled: CompiledRule,
}

impl LoadedWorkflow {
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn format(&self) -> WorkflowFileFormat {
        self.format
    }

    pub fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }

    pub fn compiled(&self) -> &CompiledRule {
        &self.compiled
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowDirectory {
    root: PathBuf,
    authored_sha256: [u8; 32],
    workflows: BTreeMap<String, LoadedWorkflow>,
    entry_count: usize,
}

impl WorkflowDirectory {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, WorkflowFilesError> {
        let root = root.as_ref();
        let root_directory = open_root_directory(root)?;
        let mut files = Vec::new();
        let mut limits = DiscoveryLimits {
            entries: 0,
            directories: 1,
        };
        collect_workflow_files(&root_directory, Path::new(""), 0, &mut limits, &mut files)?;
        files.sort_by(|left, right| left.0.cmp(&right.0));
        if files.len() > MAX_WORKFLOW_FILES {
            return Err(WorkflowFilesError::TooManyFiles {
                actual: files.len(),
                maximum: MAX_WORKFLOW_FILES,
            });
        }

        let mut workflows = BTreeMap::new();
        let mut first_paths = BTreeMap::new();
        let mut directory_hasher = Sha256::new();
        directory_hasher.update(DIRECTORY_DIGEST_DOMAIN);
        for (relative_path, format, bytes) in files {
            let source_sha256: [u8; 32] = Sha256::digest(&bytes).into();
            let compiled = compile_workflow(&relative_path, format, &bytes)?;
            let rule_id = compiled.rule_id().to_owned();
            if let Some(first_path) = first_paths.insert(rule_id.clone(), relative_path.clone()) {
                return Err(WorkflowFilesError::DuplicateRuleId {
                    rule_id,
                    first_path,
                    second_path: relative_path,
                });
            }
            hash_directory_entry(&mut directory_hasher, &relative_path, &source_sha256);
            workflows.insert(
                rule_id,
                LoadedWorkflow {
                    relative_path,
                    format,
                    source_sha256,
                    compiled,
                },
            );
        }

        Ok(Self {
            root: root.to_path_buf(),
            authored_sha256: directory_hasher.finalize().into(),
            workflows,
            entry_count: limits.entries,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn authored_sha256(&self) -> [u8; 32] {
        self.authored_sha256
    }

    pub fn workflows(&self) -> &BTreeMap<String, LoadedWorkflow> {
        &self.workflows
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowValidation {
    pub rule_id: String,
    pub revision: u64,
    pub enabled: bool,
    pub semantic_sha256: String,
}

#[derive(Debug)]
pub struct WorkflowRepository {
    root: PathBuf,
    authoring_root: PathBuf,
    current: RwLock<Arc<WorkflowDirectory>>,
    mutation: Mutex<()>,
}

impl WorkflowRepository {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, WorkflowFilesError> {
        let root = root.as_ref().to_path_buf();
        Self::open_with_authoring_root(&root, &root)
    }

    pub fn open_with_authoring_root(
        root: impl AsRef<Path>,
        authoring_root: impl AsRef<Path>,
    ) -> Result<Self, WorkflowFilesError> {
        let root = root.as_ref().to_path_buf();
        let authoring_root = authoring_root.as_ref().to_path_buf();
        let _ = open_root_directory(&authoring_root)?;
        let current = Arc::new(WorkflowDirectory::load(&root)?);
        Ok(Self {
            root,
            authoring_root,
            current: RwLock::new(current),
            mutation: Mutex::new(()),
        })
    }

    /// Returns the immutable workflow set loaded for the current process epoch.
    /// Authored file mutations do not change this snapshot.
    pub fn snapshot(&self) -> Result<Arc<WorkflowDirectory>, WorkflowMutationError> {
        self.current
            .read()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| WorkflowMutationError::StatePoisoned)
    }

    /// Reads and validates the current authored directory without changing the loaded snapshot.
    pub fn authored_snapshot(&self) -> Result<Arc<WorkflowDirectory>, WorkflowMutationError> {
        let _mutation = self
            .mutation
            .lock()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        let authoring_root = open_root_directory(&self.authoring_root)?;
        let _lock = acquire_authoring_lock(&authoring_root)?;
        Ok(Arc::new(WorkflowDirectory::load(&self.root)?))
    }

    /// Reads exact authored source through the same confined, bounded path as discovery.
    pub fn read_source(&self, relative_path: &str) -> Result<Vec<u8>, WorkflowMutationError> {
        let relative = validate_mutation_path(relative_path)?;
        workflow_format(&relative).ok_or_else(|| WorkflowMutationError::UnsupportedExtension {
            path: relative.clone(),
        })?;
        let _mutation = self
            .mutation
            .lock()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        let root = open_root_directory(&self.root)?;
        let authoring_root = open_root_directory(&self.authoring_root)?;
        let _lock = acquire_authoring_lock(&authoring_root)?;
        read_source_locked(&root, &relative)
    }

    /// Returns one authored workflow snapshot and its exact coherent source.
    pub fn authored_workflow_source(
        &self,
        rule_id: &str,
    ) -> Result<(Arc<WorkflowDirectory>, Vec<u8>), WorkflowMutationError> {
        let _mutation = self
            .mutation
            .lock()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        let root = open_root_directory(&self.root)?;
        let authoring_root = open_root_directory(&self.authoring_root)?;
        let _lock = acquire_authoring_lock(&authoring_root)?;
        let authored = Arc::new(WorkflowDirectory::load(&self.root)?);
        let workflow = authored.workflows().get(rule_id).ok_or_else(|| {
            WorkflowMutationError::WorkflowNotFound {
                rule_id: rule_id.to_owned(),
            }
        })?;
        let relative = PathBuf::from(workflow.relative_path());
        let source = read_source_locked(&root, &relative)?;
        if <[u8; 32]>::from(Sha256::digest(&source)) != workflow.source_sha256() {
            return Err(WorkflowMutationError::Conflict);
        }
        Ok((authored, source))
    }

    /// Validates and compiles one candidate source without reading or mutating the directory.
    pub fn validate_source(
        relative_path: &str,
        source: &[u8],
    ) -> Result<WorkflowValidation, WorkflowMutationError> {
        if source.len() > HARD_MAX_JSON_BYTES {
            return Err(WorkflowFilesError::FileTooLarge {
                path: PathBuf::from(relative_path),
                actual: source.len(),
                maximum: HARD_MAX_JSON_BYTES,
            }
            .into());
        }
        let relative = validate_mutation_path(relative_path)?;
        let format = workflow_format(&relative).ok_or_else(|| {
            WorkflowMutationError::UnsupportedExtension {
                path: relative.clone(),
            }
        })?;
        let compiled = compile_workflow(relative_path, format, source)?;
        Ok(WorkflowValidation {
            rule_id: compiled.rule_id().to_owned(),
            revision: compiled.revision(),
            enabled: compiled.enabled(),
            semantic_sha256: compiled.semantic_sha256().to_owned(),
        })
    }

    /// Explicitly replaces the loaded snapshot after complete directory validation.
    /// Phase 1 calls this only while constructing a process epoch, never from MCP.
    pub fn reload(&self) -> Result<Arc<WorkflowDirectory>, WorkflowMutationError> {
        let _mutation = self
            .mutation
            .lock()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        let authoring_root = open_root_directory(&self.authoring_root)?;
        let _lock = acquire_authoring_lock(&authoring_root)?;
        let candidate = Arc::new(WorkflowDirectory::load(&self.root)?);
        let mut current = self
            .current
            .write()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        *current = candidate.clone();
        Ok(candidate)
    }

    /// Atomically writes authored source and returns the validated authored set.
    /// The process-loaded snapshot remains unchanged until an explicit reload or restart.
    pub fn put(
        &self,
        relative_path: &str,
        expected_sha256: Option<[u8; 32]>,
        source: &[u8],
    ) -> Result<Arc<WorkflowDirectory>, WorkflowMutationError> {
        let validation = Self::validate_source(relative_path, source)?;
        let relative = validate_mutation_path(relative_path)?;
        let _mutation = self
            .mutation
            .lock()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        let source_sha256: [u8; 32] = Sha256::digest(source).into();

        let root_directory = open_root_directory(&self.root)?;
        let authoring_root = open_root_directory(&self.authoring_root)?;
        let _lock = acquire_authoring_lock(&authoring_root)?;
        let current = WorkflowDirectory::load(&self.root)?;
        check_snapshot_digest(&current, relative_path, expected_sha256)?;
        if expected_sha256.is_none() {
            check_create_capacity(current.workflows().len(), current.entry_count)?;
        }
        if let Some(existing) = current.workflows().get(&validation.rule_id)
            && existing.relative_path() != relative_path
        {
            return Err(WorkflowMutationError::DuplicateRuleId {
                rule_id: validation.rule_id.clone(),
                existing_path: existing.relative_path().to_owned(),
            });
        }

        let (parent, file_name) = open_parent_directory(&root_directory, &relative)?;
        check_opened_target_digest(&parent, &file_name, expected_sha256)?;
        replace_file(&parent, &file_name, source, expected_sha256)?;
        let candidate = Arc::new(
            WorkflowDirectory::load(&self.root)
                .map_err(|source| WorkflowMutationError::AuthoredButNotLoaded { source })?,
        );
        let written = candidate
            .workflows()
            .get(&validation.rule_id)
            .ok_or_else(|| WorkflowMutationError::AuthoredChangedAfterCommit {
                path: relative.clone(),
            })?;
        if written.relative_path() != relative_path || written.source_sha256() != source_sha256 {
            return Err(WorkflowMutationError::AuthoredChangedAfterCommit { path: relative });
        }
        Ok(candidate)
    }

    /// Changes only the authored enablement flag using deterministic serialization.
    /// The process-loaded snapshot remains unchanged until restart.
    pub fn set_enabled(
        &self,
        relative_path: &str,
        expected_sha256: [u8; 32],
        enabled: bool,
    ) -> Result<Arc<WorkflowDirectory>, WorkflowMutationError> {
        let relative = validate_mutation_path(relative_path)?;
        let format = workflow_format(&relative).ok_or_else(|| {
            WorkflowMutationError::UnsupportedExtension {
                path: relative.clone(),
            }
        })?;
        let _mutation = self
            .mutation
            .lock()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        let root = open_root_directory(&self.root)?;
        let authoring_root = open_root_directory(&self.authoring_root)?;
        let _lock = acquire_authoring_lock(&authoring_root)?;
        let current = WorkflowDirectory::load(&self.root)?;
        check_snapshot_digest(&current, relative_path, Some(expected_sha256))?;
        let source = match read_source_locked(&root, &relative) {
            Err(WorkflowMutationError::Io { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                return Err(WorkflowMutationError::Conflict);
            }
            result => result?,
        };
        if <[u8; 32]>::from(Sha256::digest(&source)) != expected_sha256 {
            return Err(WorkflowMutationError::Conflict);
        }
        let validation = Self::validate_source(relative_path, &source)?;
        let mut document = decoded_document_after_validation(relative_path, format, &source)?;
        document.enabled = enabled;
        let encoded = match format {
            WorkflowFileFormat::Json => {
                let mut bytes = serde_json::to_vec_pretty(&document)
                    .expect("validated RuleDocument always serializes as JSON");
                bytes.push(b'\n');
                bytes
            }
            WorkflowFileFormat::Yaml => yaml_serde::to_string(&document)
                .expect("validated RuleDocument always serializes as YAML")
                .into_bytes(),
        };
        let encoded_sha256: [u8; 32] = Sha256::digest(&encoded).into();
        let (parent, file_name) = open_parent_directory(&root, &relative)?;
        replace_file(&parent, &file_name, &encoded, Some(expected_sha256))?;
        let candidate = Arc::new(
            WorkflowDirectory::load(&self.root)
                .map_err(|source| WorkflowMutationError::AuthoredButNotLoaded { source })?,
        );
        let written = candidate
            .workflows()
            .get(&validation.rule_id)
            .ok_or_else(|| WorkflowMutationError::AuthoredChangedAfterCommit {
                path: relative.clone(),
            })?;
        if written.relative_path() != relative_path || written.source_sha256() != encoded_sha256 {
            return Err(WorkflowMutationError::AuthoredChangedAfterCommit { path: relative });
        }
        Ok(candidate)
    }

    /// Atomically removes authored source and returns the validated authored set.
    /// The process-loaded snapshot remains unchanged until an explicit reload or restart.
    pub fn remove(
        &self,
        relative_path: &str,
        expected_sha256: [u8; 32],
    ) -> Result<Arc<WorkflowDirectory>, WorkflowMutationError> {
        let relative = validate_mutation_path(relative_path)?;
        let _mutation = self
            .mutation
            .lock()
            .map_err(|_| WorkflowMutationError::StatePoisoned)?;
        workflow_format(&relative).ok_or_else(|| WorkflowMutationError::UnsupportedExtension {
            path: relative.clone(),
        })?;
        let root_directory = open_root_directory(&self.root)?;
        let authoring_root = open_root_directory(&self.authoring_root)?;
        let _lock = acquire_authoring_lock(&authoring_root)?;
        let current = WorkflowDirectory::load(&self.root)?;
        check_snapshot_digest(&current, relative_path, Some(expected_sha256))?;
        let (parent, file_name) = open_parent_directory(&root_directory, &relative)?;
        check_opened_target_digest(&parent, &file_name, Some(expected_sha256))?;
        parent
            .remove_file(&file_name)
            .map_err(|source| WorkflowMutationError::Io {
                path: relative.clone(),
                source,
            })?;
        sync_directory(&parent).map_err(WorkflowMutationError::CommitIndeterminate)?;
        let candidate = Arc::new(
            WorkflowDirectory::load(&self.root)
                .map_err(|source| WorkflowMutationError::AuthoredButNotLoaded { source })?,
        );
        if candidate
            .workflows()
            .values()
            .any(|workflow| workflow.relative_path() == relative_path)
        {
            return Err(WorkflowMutationError::AuthoredChangedAfterCommit { path: relative });
        }
        Ok(candidate)
    }
}

#[derive(Debug, Error)]
pub enum WorkflowMutationError {
    #[error(transparent)]
    Files(#[from] WorkflowFilesError),
    #[error("workflow path {path:?} must be a normalized relative file path")]
    InvalidPath { path: PathBuf },
    #[error("workflow path {path:?} must end in .json, .yaml, or .yml")]
    UnsupportedExtension { path: PathBuf },
    #[error("workflow source changed since it was read")]
    Conflict,
    #[error("workflow rule ID {rule_id:?} was not found")]
    WorkflowNotFound { rule_id: String },
    #[error("another authored-file mutation is in progress")]
    Busy,
    #[error("workflow rule ID {rule_id} already exists at {existing_path:?}")]
    DuplicateRuleId {
        rule_id: String,
        existing_path: String,
    },
    #[error("workflow path {path:?} could not be mutated")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("workflow file changed but the containing-directory sync failed")]
    CommitIndeterminate(#[source] std::io::Error),
    #[error("workflow file changed but the authored directory did not reload")]
    AuthoredButNotLoaded {
        #[source]
        source: WorkflowFilesError,
    },
    #[error("workflow path {path:?} changed again after the authored mutation committed")]
    AuthoredChangedAfterCommit { path: PathBuf },
    #[error("workflow snapshot lock is poisoned")]
    StatePoisoned,
}

#[derive(Debug, Error)]
pub enum WorkflowFilesError {
    #[error("workflow path {path:?} could not be accessed")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("workflow root {path:?} is not a directory")]
    NotDirectory { path: PathBuf },
    #[error("workflow root {path:?} contains a parent or unsupported path component")]
    InvalidRootPath { path: PathBuf },
    #[error("workflow path {path:?} is a symlink")]
    Symlink { path: PathBuf },
    #[error("workflow path {path:?} is not a regular file or directory")]
    UnsupportedFileType { path: PathBuf },
    #[error("workflow path {path:?} is not valid UTF-8")]
    NonUtf8Path { path: PathBuf },
    #[error("workflow directory depth {actual} exceeds {maximum}")]
    DirectoryTooDeep { actual: usize, maximum: usize },
    #[error("workflow directory contains {actual} files, exceeding {maximum}")]
    TooManyFiles { actual: usize, maximum: usize },
    #[error("workflow tree contains {actual} directories, exceeding {maximum}")]
    TooManyDirectories { actual: usize, maximum: usize },
    #[error("workflow tree contains {actual} entries, exceeding {maximum}")]
    TooManyEntries { actual: usize, maximum: usize },
    #[error("workflow file {path:?} is {actual} bytes, exceeding {maximum}")]
    FileTooLarge {
        path: PathBuf,
        actual: usize,
        maximum: usize,
    },
    #[error("workflow file {path:?} is not valid YAML: {reason}")]
    InvalidYaml { path: String, reason: String },
    #[error("workflow file {path:?} uses unsupported YAML at {location}: {reason}")]
    UnsupportedYaml {
        path: String,
        location: String,
        reason: &'static str,
    },
    #[error("workflow file {path:?} is invalid")]
    InvalidRule {
        path: String,
        #[source]
        source: RuleJsonError,
    },
    #[error("workflow rule ID {rule_id} appears in both {first_path:?} and {second_path:?}")]
    DuplicateRuleId {
        rule_id: String,
        first_path: String,
        second_path: String,
    },
}

struct DiscoveryLimits {
    entries: usize,
    directories: usize,
}

fn open_root_directory(root: &Path) -> Result<Dir, WorkflowFilesError> {
    #[cfg(unix)]
    {
        let absolute = if root.is_absolute() {
            root.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|source| WorkflowFilesError::Io {
                    path: root.to_path_buf(),
                    source,
                })?
                .join(root)
        };
        let mut directory = Dir::open_ambient_dir("/", ambient_authority()).map_err(|source| {
            WorkflowFilesError::Io {
                path: PathBuf::from("/"),
                source,
            }
        })?;
        let mut traversed = PathBuf::from("/");
        for component in absolute.components() {
            match component {
                Component::RootDir | Component::CurDir => {}
                Component::Normal(piece) => {
                    traversed.push(piece);
                    if directory
                        .symlink_metadata(piece)
                        .map_err(|source| WorkflowFilesError::Io {
                            path: traversed.clone(),
                            source,
                        })?
                        .file_type()
                        .is_symlink()
                    {
                        return Err(WorkflowFilesError::Symlink { path: traversed });
                    }
                    directory = directory.open_dir_nofollow(piece).map_err(|source| {
                        WorkflowFilesError::Io {
                            path: traversed.clone(),
                            source,
                        }
                    })?;
                }
                Component::ParentDir | Component::Prefix(_) => {
                    return Err(WorkflowFilesError::InvalidRootPath {
                        path: root.to_path_buf(),
                    });
                }
            }
        }
        Ok(directory)
    }
    #[cfg(not(unix))]
    {
        let metadata =
            std::fs::symlink_metadata(root).map_err(|source| WorkflowFilesError::Io {
                path: root.to_path_buf(),
                source,
            })?;
        if metadata.file_type().is_symlink() {
            return Err(WorkflowFilesError::Symlink {
                path: root.to_path_buf(),
            });
        }
        if !metadata.is_dir() {
            return Err(WorkflowFilesError::NotDirectory {
                path: root.to_path_buf(),
            });
        }
        Dir::open_ambient_dir(root, ambient_authority()).map_err(|source| WorkflowFilesError::Io {
            path: root.to_path_buf(),
            source,
        })
    }
}

fn collect_workflow_files(
    directory: &Dir,
    relative_directory: &Path,
    depth: usize,
    limits: &mut DiscoveryLimits,
    files: &mut Vec<(String, WorkflowFileFormat, Vec<u8>)>,
) -> Result<(), WorkflowFilesError> {
    if depth > MAX_WORKFLOW_DIRECTORY_DEPTH {
        return Err(WorkflowFilesError::DirectoryTooDeep {
            actual: depth,
            maximum: MAX_WORKFLOW_DIRECTORY_DEPTH,
        });
    }
    let read_dir = directory
        .entries()
        .map_err(|source| WorkflowFilesError::Io {
            path: relative_directory.to_path_buf(),
            source,
        })?;
    let mut entries = Vec::new();
    for entry in read_dir {
        limits.entries += 1;
        if limits.entries > MAX_WORKFLOW_ENTRIES {
            return Err(WorkflowFilesError::TooManyEntries {
                actual: limits.entries,
                maximum: MAX_WORKFLOW_ENTRIES,
            });
        }
        let entry = entry.map_err(|source| WorkflowFilesError::Io {
            path: relative_directory.to_path_buf(),
            source,
        })?;
        let name =
            entry
                .file_name()
                .into_string()
                .map_err(|name| WorkflowFilesError::NonUtf8Path {
                    path: relative_directory.join(name),
                })?;
        entries.push((name, entry));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    for (name, entry) in entries {
        let relative_path = relative_directory.join(&name);
        let file_type = entry.file_type().map_err(|source| WorkflowFilesError::Io {
            path: relative_path.clone(),
            source,
        })?;
        if file_type.is_symlink() {
            return Err(WorkflowFilesError::Symlink {
                path: relative_path,
            });
        }
        if file_type.is_dir() {
            limits.directories += 1;
            if limits.directories > MAX_WORKFLOW_DIRECTORIES {
                return Err(WorkflowFilesError::TooManyDirectories {
                    actual: limits.directories,
                    maximum: MAX_WORKFLOW_DIRECTORIES,
                });
            }
            let child =
                directory
                    .open_dir_nofollow(&name)
                    .map_err(|source| WorkflowFilesError::Io {
                        path: relative_path.clone(),
                        source,
                    })?;
            collect_workflow_files(&child, &relative_path, depth + 1, limits, files)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(WorkflowFilesError::UnsupportedFileType {
                path: relative_path,
            });
        }
        let Some(format) = workflow_format(&relative_path) else {
            continue;
        };
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        #[cfg(unix)]
        options.custom_flags(libc::O_NONBLOCK);
        let file =
            directory
                .open_with(&name, &options)
                .map_err(|source| WorkflowFilesError::Io {
                    path: relative_path.clone(),
                    source,
                })?;
        let bytes = read_bounded_file(file, &relative_path)?;
        let relative_path = normalized_relative_path(&relative_path)?;
        files.push((relative_path, format, bytes));
        if files.len() > MAX_WORKFLOW_FILES {
            return Err(WorkflowFilesError::TooManyFiles {
                actual: files.len(),
                maximum: MAX_WORKFLOW_FILES,
            });
        }
    }
    Ok(())
}

fn normalized_relative_path(path: &Path) -> Result<String, WorkflowFilesError> {
    let mut pieces = Vec::new();
    for component in path.components() {
        let Component::Normal(piece) = component else {
            return Err(WorkflowFilesError::UnsupportedFileType {
                path: path.to_path_buf(),
            });
        };
        pieces.push(
            piece
                .to_str()
                .ok_or_else(|| WorkflowFilesError::NonUtf8Path {
                    path: path.to_path_buf(),
                })?,
        );
    }
    Ok(pieces.join("/"))
}

fn workflow_format(path: &Path) -> Option<WorkflowFileFormat> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => Some(WorkflowFileFormat::Json),
        Some("yaml" | "yml") => Some(WorkflowFileFormat::Yaml),
        _ => None,
    }
}

fn read_bounded_file(file: cap_std::fs::File, path: &Path) -> Result<Vec<u8>, WorkflowFilesError> {
    let metadata = file.metadata().map_err(|source| WorkflowFilesError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(WorkflowFilesError::UnsupportedFileType {
            path: path.to_path_buf(),
        });
    }
    let declared = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
    if declared > HARD_MAX_JSON_BYTES {
        return Err(WorkflowFilesError::FileTooLarge {
            path: path.to_path_buf(),
            actual: declared,
            maximum: HARD_MAX_JSON_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(declared.saturating_add(1));
    file.take((HARD_MAX_JSON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| WorkflowFilesError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() > HARD_MAX_JSON_BYTES {
        return Err(WorkflowFilesError::FileTooLarge {
            path: path.to_path_buf(),
            actual: bytes.len(),
            maximum: HARD_MAX_JSON_BYTES,
        });
    }
    Ok(bytes)
}

fn decoded_document_after_validation(
    relative_path: &str,
    format: WorkflowFileFormat,
    bytes: &[u8],
) -> Result<RuleDocument, WorkflowFilesError> {
    match format {
        WorkflowFileFormat::Json => {
            serde_json::from_slice(bytes).map_err(|error| WorkflowFilesError::InvalidRule {
                path: relative_path.to_owned(),
                source: RuleJsonError::InvalidJson(error.to_string()),
            })
        }
        WorkflowFileFormat::Yaml => {
            yaml_serde::from_slice(bytes).map_err(|error| WorkflowFilesError::InvalidYaml {
                path: relative_path.to_owned(),
                reason: error.to_string(),
            })
        }
    }
}

fn compile_workflow(
    relative_path: &str,
    format: WorkflowFileFormat,
    bytes: &[u8],
) -> Result<CompiledRule, WorkflowFilesError> {
    let json = match format {
        WorkflowFileFormat::Json => bytes.to_vec(),
        WorkflowFileFormat::Yaml => {
            strict_yaml_preflight(relative_path, bytes)?;
            let value: YamlValue =
                yaml_serde::from_slice(bytes).map_err(|error| WorkflowFilesError::InvalidYaml {
                    path: relative_path.to_owned(),
                    reason: error.to_string(),
                })?;
            validate_yaml_value(relative_path, &value, "$", 1)?;
            let document: RuleDocument =
                yaml_serde::from_value(value).map_err(|error| WorkflowFilesError::InvalidYaml {
                    path: relative_path.to_owned(),
                    reason: error.to_string(),
                })?;
            serde_json::to_vec(&document).expect("RuleDocument always serializes as JSON")
        }
    };
    compile_rule_json(&json).map_err(|source| WorkflowFilesError::InvalidRule {
        path: relative_path.to_owned(),
        source,
    })
}

fn strict_yaml_preflight(path: &str, bytes: &[u8]) -> Result<(), WorkflowFilesError> {
    let source = std::str::from_utf8(bytes).map_err(|error| WorkflowFilesError::InvalidYaml {
        path: path.to_owned(),
        reason: error.to_string(),
    })?;
    let mut parser = YamlParser::new_from_str(source);
    let mut event_count = 0usize;
    let mut document_count = 0usize;
    let mut depth = 0usize;
    loop {
        let (event, marker) =
            parser
                .next_token()
                .map_err(|error| WorkflowFilesError::InvalidYaml {
                    path: path.to_owned(),
                    reason: error.to_string(),
                })?;
        event_count += 1;
        if event_count > MAX_YAML_EVENTS {
            return Err(WorkflowFilesError::UnsupportedYaml {
                path: path.to_owned(),
                location: format!("line {} column {}", marker.line(), marker.col()),
                reason: "YAML event count exceeds the Rule JSON bound",
            });
        }
        let location = || format!("line {} column {}", marker.line(), marker.col());
        match event {
            YamlEvent::DocumentStart => {
                document_count += 1;
                if document_count > 1 {
                    return Err(WorkflowFilesError::UnsupportedYaml {
                        path: path.to_owned(),
                        location: location(),
                        reason: "each workflow file must contain exactly one YAML document",
                    });
                }
            }
            YamlEvent::Alias(_) => {
                return Err(WorkflowFilesError::UnsupportedYaml {
                    path: path.to_owned(),
                    location: location(),
                    reason: "YAML aliases are not supported",
                });
            }
            YamlEvent::Scalar(_, _, anchor, tag) => {
                reject_yaml_anchor_or_tag(path, location(), anchor, tag.is_some())?;
            }
            YamlEvent::SequenceStart(anchor, tag) | YamlEvent::MappingStart(anchor, tag) => {
                reject_yaml_anchor_or_tag(path, location(), anchor, tag.is_some())?;
                depth += 1;
                if depth > HARD_MAX_JSON_DEPTH {
                    return Err(WorkflowFilesError::UnsupportedYaml {
                        path: path.to_owned(),
                        location: location(),
                        reason: "nesting exceeds the Rule JSON bound",
                    });
                }
            }
            YamlEvent::SequenceEnd | YamlEvent::MappingEnd => {
                depth = depth.saturating_sub(1);
            }
            YamlEvent::StreamEnd => break,
            YamlEvent::Nothing | YamlEvent::StreamStart | YamlEvent::DocumentEnd => {}
        }
    }
    if document_count != 1 {
        return Err(WorkflowFilesError::InvalidYaml {
            path: path.to_owned(),
            reason: "workflow YAML must contain exactly one document".to_owned(),
        });
    }
    Ok(())
}

fn reject_yaml_anchor_or_tag(
    path: &str,
    location: String,
    anchor: usize,
    tagged: bool,
) -> Result<(), WorkflowFilesError> {
    if anchor != 0 {
        return Err(WorkflowFilesError::UnsupportedYaml {
            path: path.to_owned(),
            location,
            reason: "YAML anchors are not supported",
        });
    }
    if tagged {
        return Err(WorkflowFilesError::UnsupportedYaml {
            path: path.to_owned(),
            location,
            reason: "YAML tags are not supported",
        });
    }
    Ok(())
}

fn validate_yaml_value(
    path: &str,
    value: &YamlValue,
    location: &str,
    depth: usize,
) -> Result<(), WorkflowFilesError> {
    if depth > HARD_MAX_JSON_DEPTH {
        return Err(WorkflowFilesError::UnsupportedYaml {
            path: path.to_owned(),
            location: location.to_owned(),
            reason: "nesting exceeds the Rule JSON bound",
        });
    }
    match value {
        YamlValue::Null | YamlValue::Bool(_) => Ok(()),
        YamlValue::Number(number) if !number.is_f64() => Ok(()),
        YamlValue::Number(_) => Err(WorkflowFilesError::UnsupportedYaml {
            path: path.to_owned(),
            location: location.to_owned(),
            reason: "floating-point values are not part of Rule JSON",
        }),
        YamlValue::String(string) if string.len() <= HARD_MAX_STRING_BYTES => Ok(()),
        YamlValue::String(_) => Err(WorkflowFilesError::UnsupportedYaml {
            path: path.to_owned(),
            location: location.to_owned(),
            reason: "string exceeds the Rule JSON bound",
        }),
        YamlValue::Sequence(values) => {
            if values.len() > HARD_MAX_COLLECTION_ITEMS {
                return Err(WorkflowFilesError::UnsupportedYaml {
                    path: path.to_owned(),
                    location: location.to_owned(),
                    reason: "sequence exceeds the Rule JSON bound",
                });
            }
            for (index, value) in values.iter().enumerate() {
                validate_yaml_value(path, value, &format!("{location}[{index}]"), depth + 1)?;
            }
            Ok(())
        }
        YamlValue::Mapping(values) => {
            if values.len() > HARD_MAX_COLLECTION_ITEMS {
                return Err(WorkflowFilesError::UnsupportedYaml {
                    path: path.to_owned(),
                    location: location.to_owned(),
                    reason: "mapping exceeds the Rule JSON bound",
                });
            }
            for (key, value) in values {
                let YamlValue::String(key) = key else {
                    return Err(WorkflowFilesError::UnsupportedYaml {
                        path: path.to_owned(),
                        location: location.to_owned(),
                        reason: "mapping keys must be strings",
                    });
                };
                validate_yaml_value(path, value, &format!("{location}.{key}"), depth + 1)?;
            }
            Ok(())
        }
        YamlValue::Tagged(_) => Err(WorkflowFilesError::UnsupportedYaml {
            path: path.to_owned(),
            location: location.to_owned(),
            reason: "YAML tags are not supported",
        }),
    }
}

fn validate_mutation_path(path: &str) -> Result<PathBuf, WorkflowMutationError> {
    let candidate = PathBuf::from(path);
    let structurally_valid = !candidate.as_os_str().is_empty()
        && path.len() <= MAX_WORKFLOW_RELATIVE_PATH_BYTES
        && !candidate.is_absolute()
        && !path.contains('\\')
        && candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !structurally_valid || normalized_relative_path(&candidate).ok().as_deref() != Some(path) {
        return Err(WorkflowMutationError::InvalidPath { path: candidate });
    }
    Ok(candidate)
}

fn check_create_capacity(
    workflow_count: usize,
    entry_count: usize,
) -> Result<(), WorkflowMutationError> {
    if workflow_count >= MAX_WORKFLOW_FILES {
        return Err(WorkflowFilesError::TooManyFiles {
            actual: workflow_count.saturating_add(1),
            maximum: MAX_WORKFLOW_FILES,
        }
        .into());
    }
    if entry_count >= MAX_WORKFLOW_ENTRIES {
        return Err(WorkflowFilesError::TooManyEntries {
            actual: entry_count.saturating_add(1),
            maximum: MAX_WORKFLOW_ENTRIES,
        }
        .into());
    }
    Ok(())
}

fn check_snapshot_digest(
    snapshot: &WorkflowDirectory,
    relative_path: &str,
    expected_sha256: Option<[u8; 32]>,
) -> Result<(), WorkflowMutationError> {
    let actual = snapshot
        .workflows()
        .values()
        .find(|workflow| workflow.relative_path() == relative_path)
        .map(LoadedWorkflow::source_sha256);
    if actual == expected_sha256 {
        Ok(())
    } else {
        Err(WorkflowMutationError::Conflict)
    }
}

fn acquire_authoring_lock(root: &Dir) -> Result<std::fs::File, WorkflowMutationError> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(true)
        .follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.custom_flags(libc::O_NONBLOCK);
    let file = root
        .open_with(".litehouse-authoring.lock", &options)
        .map_err(|source| WorkflowMutationError::Io {
            path: PathBuf::from(".litehouse-authoring.lock"),
            source,
        })?;
    if !file
        .metadata()
        .map_err(|source| WorkflowMutationError::Io {
            path: PathBuf::from(".litehouse-authoring.lock"),
            source,
        })?
        .is_file()
    {
        return Err(WorkflowMutationError::InvalidPath {
            path: PathBuf::from(".litehouse-authoring.lock"),
        });
    }
    let file = file.into_std();
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Err(WorkflowMutationError::Busy),
        Err(std::fs::TryLockError::Error(source)) => {
            return Err(WorkflowMutationError::Io {
                path: PathBuf::from(".litehouse-authoring.lock"),
                source,
            });
        }
    }
    Ok(file)
}

fn open_parent_directory(
    root: &Dir,
    relative_path: &Path,
) -> Result<(Dir, String), WorkflowMutationError> {
    let file_name = relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| WorkflowMutationError::InvalidPath {
            path: relative_path.to_path_buf(),
        })?
        .to_owned();
    let mut directory = Dir::reopen_dir(root).map_err(|source| WorkflowMutationError::Io {
        path: PathBuf::new(),
        source,
    })?;
    if let Some(parent) = relative_path.parent() {
        let mut traversed = PathBuf::new();
        for component in parent.components() {
            let Component::Normal(piece) = component else {
                return Err(WorkflowMutationError::InvalidPath {
                    path: relative_path.to_path_buf(),
                });
            };
            traversed.push(piece);
            directory =
                directory
                    .open_dir_nofollow(piece)
                    .map_err(|source| WorkflowMutationError::Io {
                        path: traversed.clone(),
                        source,
                    })?;
        }
    }
    Ok((directory, file_name))
}

fn read_source_locked(root: &Dir, relative: &Path) -> Result<Vec<u8>, WorkflowMutationError> {
    let (parent, file_name) = open_parent_directory(root, relative)?;
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.custom_flags(libc::O_NONBLOCK);
    let file =
        parent
            .open_with(&file_name, &options)
            .map_err(|source| WorkflowMutationError::Io {
                path: relative.to_path_buf(),
                source,
            })?;
    read_bounded_file(file, relative).map_err(Into::into)
}

fn check_opened_target_digest(
    parent: &Dir,
    file_name: &str,
    expected_sha256: Option<[u8; 32]>,
) -> Result<(), WorkflowMutationError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.custom_flags(libc::O_NONBLOCK);
    match parent.open_with(file_name, &options) {
        Ok(file) => {
            let bytes = read_bounded_file(file, Path::new(file_name))?;
            if expected_sha256 == Some(Sha256::digest(&bytes).into()) {
                Ok(())
            } else {
                Err(WorkflowMutationError::Conflict)
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && expected_sha256.is_none() => {
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(WorkflowMutationError::Conflict)
        }
        Err(source) => Err(WorkflowMutationError::Io {
            path: PathBuf::from(file_name),
            source,
        }),
    }
}

fn replace_file(
    parent: &Dir,
    file_name: &str,
    source: &[u8],
    expected_sha256: Option<[u8; 32]>,
) -> Result<(), WorkflowMutationError> {
    let sequence = WORKFLOW_TEMP_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
    let temporary = format!(".{file_name}.{}.{}.tmp", std::process::id(), sequence);
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut file =
        parent
            .open_with(&temporary, &options)
            .map_err(|source| WorkflowMutationError::Io {
                path: PathBuf::from(&temporary),
                source,
            })?;
    let write_result = (|| {
        file.write_all(source)?;
        file.sync_all()?;
        drop(file);
        check_opened_target_digest(parent, file_name, expected_sha256).map_err(
            |error| match error {
                WorkflowMutationError::Io { source, .. } => source,
                WorkflowMutationError::Conflict => {
                    std::io::Error::new(std::io::ErrorKind::AlreadyExists, "workflow changed")
                }
                other => std::io::Error::other(other),
            },
        )?;
        parent.rename(&temporary, parent, file_name)
    })();
    if let Err(source) = write_result {
        let _ = parent.remove_file(&temporary);
        if source.kind() == std::io::ErrorKind::AlreadyExists {
            return Err(WorkflowMutationError::Conflict);
        }
        return Err(WorkflowMutationError::Io {
            path: PathBuf::from(file_name),
            source,
        });
    }
    sync_directory(parent).map_err(WorkflowMutationError::CommitIndeterminate)
}

fn sync_directory(directory: &Dir) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .maybe_dir(true)
        .follow(FollowSymlinks::No);
    directory.open_with(".", &options)?.sync_all()
}

fn hash_directory_entry(hasher: &mut Sha256, path: &str, source_sha256: &[u8; 32]) {
    hasher.update((path.len() as u32).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update(source_sha256);
}
