use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, ambient_authority};
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
    io::Read,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;
use yaml_rust2::parser::{Event as YamlEvent, Parser as YamlParser};
use yaml_serde::Value as YamlValue;

const MAX_WORKFLOW_FILES: usize = 256;
const MAX_WORKFLOW_DIRECTORIES: usize = 512;
const MAX_WORKFLOW_ENTRIES: usize = 4_096;
const MAX_WORKFLOW_DIRECTORY_DEPTH: usize = 16;
const MAX_YAML_EVENTS: usize = 8_192;
const DIRECTORY_DIGEST_DOMAIN: &[u8] = b"litehouse-workflow-directory-v1\0";

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

fn hash_directory_entry(hasher: &mut Sha256, path: &str, source_sha256: &[u8; 32]) {
    hasher.update((path.len() as u32).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update(source_sha256);
}
