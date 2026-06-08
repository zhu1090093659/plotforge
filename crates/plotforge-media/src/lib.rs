use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use plotforge_schema::{
    AssetKind, AssetProviderMetadata, AssetRecord, AssetReference, AssetReferenceKind,
    AssetSourceKind, ProjectData,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const ASSET_HASH_ALGORITHM: &str = "sha256";

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("asset path is not project-safe: {0}")]
    UnsafeAssetPath(String),
    #[error("asset file is empty: {0}")]
    EmptyAsset(String),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRecordInput {
    pub kind: AssetKind,
    pub source: AssetSourceKind,
    pub project_path: String,
    pub export_path: Option<String>,
    pub provider_metadata: Option<AssetProviderMetadata>,
    pub references: Vec<AssetReference>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetRegistry {
    records: BTreeMap<String, AssetRecord>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> impl Iterator<Item = &AssetRecord> {
        self.records.values()
    }

    pub fn get(&self, id: &str) -> Option<&AssetRecord> {
        self.records.get(id)
    }

    pub fn insert_bytes(
        &mut self,
        input: AssetRecordInput,
        bytes: &[u8],
    ) -> Result<String, MediaError> {
        if bytes.is_empty() {
            return Err(MediaError::EmptyAsset(input.project_path));
        }

        let project_path = AssetPath::new(&input.project_path)?;
        let export_path = match input.export_path {
            Some(path) => AssetPath::new(&path)?,
            None => project_path.clone(),
        };
        let content_hash = sha256_hex(bytes);
        let id = stable_asset_id(&input.kind, &content_hash);
        let references = unique_references(input.references);

        if let Some(record) = self.records.get_mut(&id) {
            for reference in references {
                if !record.references.contains(&reference) {
                    record.references.push(reference);
                }
            }
            record.references.sort();
            if record.provider_metadata.is_none() {
                record.provider_metadata = input.provider_metadata;
            }
            return Ok(id);
        }

        let record = AssetRecord {
            id: id.clone(),
            kind: input.kind,
            source: input.source,
            project_path: project_path.as_str().to_string(),
            export_path: export_path.as_str().to_string(),
            content_hash,
            hash_algorithm: ASSET_HASH_ALGORITHM.into(),
            byte_length: bytes.len() as u64,
            provider_metadata: input.provider_metadata,
            references,
        };
        self.records.insert(id.clone(), record);
        Ok(id)
    }

    pub fn insert_file(
        &mut self,
        project_root: impl AsRef<Path>,
        input: AssetRecordInput,
    ) -> Result<String, MediaError> {
        let relative_path = AssetPath::new(&input.project_path)?;
        let file_path = project_root.as_ref().join(relative_path.as_path());
        let bytes = fs::read(&file_path).map_io(&file_path)?;
        self.insert_bytes(input, &bytes)
    }

    pub fn register_scene_background_assets(
        &mut self,
        project_root: impl AsRef<Path>,
        project: &ProjectData,
    ) -> Result<(), MediaError> {
        let project_root = project_root.as_ref();
        for scene in &project.scenes {
            let reference = AssetReference {
                reference_kind: AssetReferenceKind::Scene,
                reference_id: scene.key.clone(),
                slot: "background_asset".into(),
            };
            self.insert_file(
                project_root,
                AssetRecordInput {
                    kind: AssetKind::Image,
                    source: AssetSourceKind::Generated,
                    project_path: scene.background_asset.clone(),
                    export_path: Some(scene.background_asset.clone()),
                    provider_metadata: None,
                    references: vec![reference],
                },
            )?;
        }
        Ok(())
    }

    pub fn reachable_records(&self) -> Vec<&AssetRecord> {
        self.records
            .values()
            .filter(|record| !record.references.is_empty())
            .collect()
    }

    pub fn exportable_paths(&self) -> Vec<String> {
        self.reachable_records()
            .into_iter()
            .map(|record| record.export_path.clone())
            .collect()
    }

    pub fn records_referenced_by(
        &self,
        reference_kind: AssetReferenceKind,
        reference_id: &str,
    ) -> Vec<&AssetRecord> {
        self.records
            .values()
            .filter(|record| {
                record.references.iter().any(|reference| {
                    reference.reference_kind == reference_kind
                        && reference.reference_id == reference_id
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AssetPath(String);

impl AssetPath {
    fn new(path: &str) -> Result<Self, MediaError> {
        let parsed = Path::new(path);
        if path.is_empty() || parsed.is_absolute() || !parsed.starts_with("assets") {
            return Err(MediaError::UnsafeAssetPath(path.into()));
        }

        for component in parsed.components() {
            if !matches!(component, Component::Normal(_)) {
                return Err(MediaError::UnsafeAssetPath(path.into()));
            }
        }

        Ok(Self(path.into()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

fn unique_references(references: Vec<AssetReference>) -> Vec<AssetReference> {
    references
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn stable_asset_id(kind: &AssetKind, content_hash: &str) -> String {
    format!("asset-{}-{}", kind.id_prefix(), &content_hash[..16])
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

trait IoContext<T> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, MediaError>;
}

impl<T> IoContext<T> for Result<T, std::io::Error> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, MediaError> {
        self.map_err(|source| MediaError::Io {
            path: path.as_ref().to_path_buf(),
            source,
        })
    }
}

trait AssetKindId {
    fn id_prefix(&self) -> &'static str;
}

impl AssetKindId for AssetKind {
    fn id_prefix(&self) -> &'static str {
        match self {
            AssetKind::Image => "image",
            AssetKind::Audio => "audio",
            AssetKind::Voice => "voice",
            AssetKind::Data => "data",
        }
    }
}

#[cfg(test)]
mod tests {
    use plotforge_schema::{
        AssetKind, AssetProviderMetadata, AssetRecord, AssetReference, AssetReferenceKind,
        AssetSourceKind,
    };
    use plotforge_storage::{create_demo_project, load_project};

    use super::{ASSET_HASH_ALGORITHM, AssetRecordInput, AssetRegistry, MediaError};

    #[test]
    fn deduplicates_assets_by_hash_and_merges_references() {
        let mut registry = AssetRegistry::new();
        let first = registry
            .insert_bytes(
                AssetRecordInput {
                    kind: AssetKind::Image,
                    source: AssetSourceKind::Generated,
                    project_path: "assets/generated/one.png".into(),
                    export_path: None,
                    provider_metadata: Some(provider_metadata()),
                    references: vec![scene_reference("scene-one")],
                },
                b"same image bytes",
            )
            .expect("insert first");
        let second = registry
            .insert_bytes(
                AssetRecordInput {
                    kind: AssetKind::Image,
                    source: AssetSourceKind::Generated,
                    project_path: "assets/generated/two.png".into(),
                    export_path: None,
                    provider_metadata: None,
                    references: vec![scene_reference("scene-two")],
                },
                b"same image bytes",
            )
            .expect("insert duplicate");

        assert_eq!(first, second);
        assert_eq!(registry.len(), 1);
        let record = registry.get(&first).expect("record");
        assert_eq!(record.references.len(), 2);
        assert_eq!(record.hash_algorithm, ASSET_HASH_ALGORITHM);
        assert_eq!(record.content_hash.len(), 64);
        assert_eq!(record.byte_length, 16);
        assert_eq!(record.export_path, "assets/generated/one.png");
        assert_eq!(
            record
                .provider_metadata
                .as_ref()
                .map(|metadata| metadata.provider.as_str()),
            Some("fake-image")
        );
    }

    #[test]
    fn rejects_unsafe_asset_paths_before_reading_or_exporting() {
        let mut registry = AssetRegistry::new();

        let error = registry
            .insert_bytes(
                AssetRecordInput {
                    kind: AssetKind::Image,
                    source: AssetSourceKind::Generated,
                    project_path: "../traces/latest.json".into(),
                    export_path: None,
                    provider_metadata: None,
                    references: vec![scene_reference("scene-one")],
                },
                b"bytes",
            )
            .expect_err("unsafe path");

        assert!(matches!(error, MediaError::UnsafeAssetPath(_)));
        assert!(registry.is_empty());
    }

    #[test]
    fn builds_scene_background_registry_with_reachable_export_paths() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_path = temp.path().join("project");
        create_demo_project(&project_path, false).expect("create demo");
        let project = load_project(&project_path).expect("load project");
        let mut registry = AssetRegistry::new();

        registry
            .register_scene_background_assets(&project_path, &project)
            .expect("register backgrounds");

        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry.exportable_paths(),
            vec!["assets/generated/court-crisis-001.png"]
        );
        let scene_records =
            registry.records_referenced_by(AssetReferenceKind::Scene, "court-crisis-001");
        assert_eq!(scene_records.len(), 1);
        let record = scene_records[0];
        assert_eq!(record.kind, AssetKind::Image);
        assert_eq!(record.source, AssetSourceKind::Generated);
        assert_eq!(record.references, vec![scene_reference("court-crisis-001")]);
        assert_eq!(record.content_hash.len(), 64);
    }

    #[test]
    fn keeps_unreferenced_assets_out_of_exportable_paths() {
        let mut registry = AssetRegistry::new();

        registry
            .insert_bytes(
                AssetRecordInput {
                    kind: AssetKind::Image,
                    source: AssetSourceKind::UserImport,
                    project_path: "assets/images/unused.png".into(),
                    export_path: None,
                    provider_metadata: None,
                    references: Vec::new(),
                },
                b"unused image bytes",
            )
            .expect("insert unreferenced");

        assert_eq!(registry.len(), 1);
        assert!(registry.reachable_records().is_empty());
        assert!(registry.exportable_paths().is_empty());
    }

    #[test]
    fn asset_record_rejects_raw_provider_response_fields() {
        let mut record = serde_json::to_value(sample_record()).expect("record json");
        record["provider_metadata"]["raw_response"] = serde_json::json!("secret provider body");

        let error = serde_json::from_value::<AssetRecord>(record)
            .expect_err("raw provider response should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    fn sample_record() -> AssetRecord {
        AssetRecord {
            id: "asset-image-test".into(),
            kind: AssetKind::Image,
            source: AssetSourceKind::Generated,
            project_path: "assets/generated/test.png".into(),
            export_path: "assets/generated/test.png".into(),
            content_hash: "0".repeat(64),
            hash_algorithm: ASSET_HASH_ALGORITHM.into(),
            byte_length: 10,
            provider_metadata: Some(provider_metadata()),
            references: vec![scene_reference("scene-one")],
        }
    }

    fn provider_metadata() -> AssetProviderMetadata {
        AssetProviderMetadata {
            provider: "fake-image".into(),
            model: Some("placeholder-v1".into()),
            request_id: Some("request-1".into()),
            prompt_hash: Some("prompt-hash".into()),
            fallback_used: false,
        }
    }

    fn scene_reference(scene_key: &str) -> AssetReference {
        AssetReference {
            reference_kind: AssetReferenceKind::Scene,
            reference_id: scene_key.into(),
            slot: "background_asset".into(),
        }
    }
}
