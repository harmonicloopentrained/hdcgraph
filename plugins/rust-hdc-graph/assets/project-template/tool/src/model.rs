use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphArtifact {
    pub root: String,
    pub generated_unix_seconds: u64,
    pub cache_version: String,
    pub nodes: Vec<NodeRecord>,
    pub edges: Vec<EdgeRecord>,
    pub files: Vec<FileSignatureRecord>,
    pub context_packs: Vec<ContextPackRecord>,
    pub summary: SummaryRecord,
    pub graph_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub path: String,
    pub line: Option<usize>,
    pub stable_hash: String,
    pub neighborhood_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub evidence_path: String,
    pub evidence_line: Option<usize>,
    pub stable_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSignatureRecord {
    pub path: String,
    pub kind: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub content_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackRecord {
    pub id: String,
    pub anchor_node_id: String,
    pub anchor_label: String,
    pub anchor_kind: String,
    pub path: String,
    pub line: Option<usize>,
    pub neighborhood_hash: String,
    pub file_hashes: Vec<String>,
    pub related_node_ids: Vec<String>,
    pub summary_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRecord {
    pub file_count: usize,
    pub node_count: usize,
    pub edge_count: usize,
    pub relation_counts: Vec<CountRecord>,
    pub hottest_files: Vec<FileHotspot>,
    pub strongest_pairs: Vec<SimilarityPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountRecord {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHotspot {
    pub path: String,
    pub node_count: usize,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityPair {
    pub left: String,
    pub right: String,
    pub similarity: f64,
}
