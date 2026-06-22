use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::hdc::{HyperVector, tokenize};
use crate::model::{
    ContextPackRecord, CountRecord, EdgeRecord, FileHotspot, FileSignatureRecord, GraphArtifact,
    NodeRecord, SimilarityPair, SummaryRecord,
};
use crate::rust_extract::{self, PipelineReference, RustExtraction};
use crate::stable_hash::{normalize_query, stable_hash_hex, stable_text_hash};
use crate::wgsl_extract::{self, WgslExtraction};

const CACHE_VERSION: &str = "det-hash-v1";
const MAX_QUERY_CACHE_ENTRIES: usize = 128;

#[derive(Debug, Default, Serialize, Deserialize)]
struct QueryCacheFile {
    cache_version: String,
    graph_signature: String,
    entries: Vec<QueryCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryCacheEntry {
    task_hash: String,
    mode: String,
    normalized_query: String,
    pack_ids: Vec<String>,
    response: String,
    created_unix_seconds: u64,
    hit_count: u64,
}

pub fn analyze_project(root: &Path) -> Result<GraphArtifact> {
    if !root.exists() {
        bail!("path does not exist: {}", root.display());
    }

    let root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root.display()))?;

    let mut relative_files = Vec::new();
    collect_source_files(&root, &root, &mut relative_files)?;
    relative_files.sort();

    let mut builder = GraphBuilder::new(&root);
    for rel in &relative_files {
        builder.ensure_file_node(rel);
    }

    for rel in &relative_files {
        let full = root.join(rel);
        let source = fs::read_to_string(&full)
            .with_context(|| format!("failed to read source file {}", full.display()))?;
        builder.record_file_content_hash(rel, stable_text_hash(&source));
        let file_node_id = builder.ensure_file_node(rel);

        match full.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => {
                let extracted = rust_extract::extract_rust(rel, &source);
                builder.ingest_rust(rel, &file_node_id, extracted);
            }
            Some("wgsl") => {
                let extracted = wgsl_extract::extract_wgsl(&source);
                builder.ingest_wgsl(rel, &file_node_id, extracted);
            }
            _ => {}
        }
    }

    builder.finalize()
}

pub fn write_outputs(artifact: &GraphArtifact, out_dir: &Path) -> Result<()> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    let graph_path = out_dir.join("graph.json");
    let summary_path = out_dir.join("SUMMARY.md");

    fs::write(
        &graph_path,
        serde_json::to_string_pretty(artifact).context("failed to serialize graph artifact")?,
    )
    .with_context(|| format!("failed to write {}", graph_path.display()))?;
    fs::write(&summary_path, render_summary(artifact))
        .with_context(|| format!("failed to write {}", summary_path.display()))?;

    Ok(())
}

pub fn query_graph(graph_path: &Path, needle: &str) -> Result<String> {
    query_graph_mode(graph_path, needle, QueryMode::Neighbors)
}

pub fn query_context(graph_path: &Path, needle: &str) -> Result<String> {
    query_graph_mode(graph_path, needle, QueryMode::Context)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueryMode {
    Neighbors,
    Context,
}

impl QueryMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Neighbors => "query",
            Self::Context => "context",
        }
    }
}

fn query_graph_mode(graph_path: &Path, needle: &str, mode: QueryMode) -> Result<String> {
    let artifact = load_graph_artifact(graph_path)?;
    let normalized_query = normalize_query(needle);
    let task_hash = stable_hash_hex([
        CACHE_VERSION,
        mode.as_str(),
        artifact.graph_signature.as_str(),
        normalized_query.as_str(),
    ]);

    let mut cache = load_query_cache(graph_path, &artifact.graph_signature);
    if let Some(entry) = cache.entries.iter_mut().find(|entry| entry.task_hash == task_hash) {
        entry.hit_count = entry.hit_count.saturating_add(1);
        let response = format!(
            "Cache: exact-hit {}\n{}",
            short_hash(&task_hash),
            entry.response
        );
        let _ = save_query_cache(graph_path, &cache);
        return Ok(response);
    }

    let matches = find_matching_nodes(&artifact, needle);
    if matches.is_empty() {
        return Ok(format!("No nodes matched `{needle}`."));
    }

    let pack_ids = matches
        .iter()
        .take(5)
        .filter_map(|item| artifact.context_packs.iter().find(|pack| pack.anchor_node_id == item.id))
        .map(|pack| pack.id.clone())
        .collect::<Vec<_>>();

    let body = match mode {
        QueryMode::Neighbors => render_query_response(&artifact, needle, &matches),
        QueryMode::Context => render_context_response(&artifact, needle, &task_hash, &matches),
    };

    cache.entries.push(QueryCacheEntry {
        task_hash: task_hash.clone(),
        mode: mode.as_str().to_string(),
        normalized_query,
        pack_ids,
        response: body.clone(),
        created_unix_seconds: unix_now(),
        hit_count: 0,
    });

    cache.entries.sort_by(|left, right| {
        right
            .created_unix_seconds
            .cmp(&left.created_unix_seconds)
            .then_with(|| right.hit_count.cmp(&left.hit_count))
    });
    cache.entries.truncate(MAX_QUERY_CACHE_ENTRIES);

    let _ = save_query_cache(graph_path, &cache);
    Ok(format!("Cache: miss-store {}\n{}", short_hash(&task_hash), body))
}

fn load_graph_artifact(graph_path: &Path) -> Result<GraphArtifact> {
    let json = fs::read_to_string(graph_path)
        .with_context(|| format!("failed to read {}", graph_path.display()))?;
    serde_json::from_str(&json).context("failed to parse graph artifact JSON")
}

fn render_query_response(artifact: &GraphArtifact, needle: &str, matches: &[NodeRecord]) -> String {
    let mut out = String::new();
    let pack_index = build_pack_index(artifact);

    out.push_str(&format!("Matches for `{needle}`:\n"));
    for node in matches.iter().take(5) {
        out.push_str(&format!(
            "- {} [{}] ({})\n",
            node.label, node.kind, node.path
        ));

        if let Some(pack) = pack_index.get(node.id.as_str()) {
            out.push_str(&format!("  pack {}\n", short_hash(&pack.id)));
            for line in pack.summary_lines.iter().take(3) {
                out.push_str(&format!("  ctx {line}\n"));
            }
        } else {
            render_neighbor_fallback(artifact, node, &mut out);
        }
    }

    out
}

fn render_context_response(artifact: &GraphArtifact, needle: &str, task_hash: &str, matches: &[NodeRecord]) -> String {
    let mut out = String::new();
    let pack_index = build_pack_index(artifact);

    out.push_str(&format!(
        "Context packs for `{needle}` ({})\n",
        short_hash(task_hash)
    ));
    for node in matches.iter().take(4) {
        out.push_str(&format!(
            "- {} [{}] ({})\n",
            node.label, node.kind, node.path
        ));
        if let Some(pack) = pack_index.get(node.id.as_str()) {
            out.push_str(&format!(
                "  pack {} nh={}\n",
                short_hash(&pack.id),
                short_hash(&pack.neighborhood_hash)
            ));
            if !pack.file_hashes.is_empty() {
                let file_hashes = pack
                    .file_hashes
                    .iter()
                    .map(|hash| short_hash(hash))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("  files {file_hashes}\n"));
            }
            for line in pack.summary_lines.iter().take(5) {
                out.push_str(&format!("  - {line}\n"));
            }
        } else {
            render_neighbor_fallback(artifact, node, &mut out);
        }
    }

    out
}

fn render_neighbor_fallback(artifact: &GraphArtifact, node: &NodeRecord, out: &mut String) {
    let Ok(node_vec) = HyperVector::from_hex(&node.signature)
        .ok_or_else(|| anyhow::anyhow!("invalid node signature"))
    else {
        return;
    };

    let mut neighbors = artifact
        .nodes
        .iter()
        .filter(|candidate| candidate.id != node.id)
        .filter_map(|candidate| {
            let vec = HyperVector::from_hex(&candidate.signature)?;
            Some((candidate, node_vec.similarity(vec)))
        })
        .collect::<Vec<_>>();

    neighbors.sort_by(|left, right| right.1.total_cmp(&left.1));
    for (candidate, sim) in neighbors.into_iter().take(3) {
        out.push_str(&format!(
            "  -> {:.3} {} [{}] ({})\n",
            sim, candidate.label, candidate.kind, candidate.path
        ));
    }
}

fn find_matching_nodes(artifact: &GraphArtifact, needle: &str) -> Vec<NodeRecord> {
    let normalized_query = normalize_query(needle);
    if normalized_query.is_empty() {
        return Vec::new();
    }

    let query_tokens = tokenize(needle).into_iter().collect::<BTreeSet<_>>();
    let query_vector = HyperVector::from_tokens(&query_tokens.iter().cloned().collect::<Vec<_>>());

    let mut scored = artifact
        .nodes
        .iter()
        .filter_map(|node| {
            let label_text = normalize_query(&node.label);
            let path_text = normalize_query(&node.path);
            let combined_tokens = tokenize(&format!("{} {}", node.label, node.path))
                .into_iter()
                .collect::<BTreeSet<_>>();

            let overlap = query_tokens.intersection(&combined_tokens).count() as f64;
            let overlap_ratio = overlap / query_tokens.len() as f64;

            let mut score = overlap_ratio;
            if label_text == normalized_query || path_text == normalized_query {
                score += 5.0;
            } else if label_text.contains(&normalized_query) || path_text.contains(&normalized_query) {
                score += 3.0;
            }

            let node_vec = HyperVector::from_hex(&node.signature)?;
            score += query_vector.similarity(node_vec);

            if score < 1.15 {
                return None;
            }

            Some((score, node.clone()))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| left.1.label.cmp(&right.1.label))
    });
    scored.into_iter().map(|(_, node)| node).collect()
}

fn build_pack_index<'a>(
    artifact: &'a GraphArtifact,
) -> HashMap<&'a str, &'a ContextPackRecord> {
    artifact
        .context_packs
        .iter()
        .map(|pack| (pack.anchor_node_id.as_str(), pack))
        .collect()
}

fn short_hash(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

fn query_cache_path(graph_path: &Path) -> PathBuf {
    graph_path.with_file_name("query-cache.json")
}

fn load_query_cache(graph_path: &Path, graph_signature: &str) -> QueryCacheFile {
    let cache_path = query_cache_path(graph_path);
    let Ok(json) = fs::read_to_string(&cache_path) else {
        return QueryCacheFile {
            cache_version: CACHE_VERSION.to_string(),
            graph_signature: graph_signature.to_string(),
            entries: Vec::new(),
        };
    };

    let Ok(cache) = serde_json::from_str::<QueryCacheFile>(&json) else {
        return QueryCacheFile {
            cache_version: CACHE_VERSION.to_string(),
            graph_signature: graph_signature.to_string(),
            entries: Vec::new(),
        };
    };

    if cache.cache_version != CACHE_VERSION || cache.graph_signature != graph_signature {
        QueryCacheFile {
            cache_version: CACHE_VERSION.to_string(),
            graph_signature: graph_signature.to_string(),
            entries: Vec::new(),
        }
    } else {
        cache
    }
}

fn save_query_cache(graph_path: &Path, cache: &QueryCacheFile) -> Result<()> {
    let cache_path = query_cache_path(graph_path);
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut merged = match fs::read_to_string(&cache_path) {
        Ok(json) => serde_json::from_str::<QueryCacheFile>(&json).unwrap_or_default(),
        Err(_) => QueryCacheFile::default(),
    };
    if merged.cache_version != CACHE_VERSION || merged.graph_signature != cache.graph_signature {
        merged = QueryCacheFile {
            cache_version: CACHE_VERSION.to_string(),
            graph_signature: cache.graph_signature.clone(),
            entries: Vec::new(),
        };
    }

    for entry in &cache.entries {
        if let Some(existing) = merged
            .entries
            .iter_mut()
            .find(|existing| existing.task_hash == entry.task_hash)
        {
            if entry.created_unix_seconds >= existing.created_unix_seconds {
                existing.mode = entry.mode.clone();
                existing.normalized_query = entry.normalized_query.clone();
                existing.pack_ids = entry.pack_ids.clone();
                existing.response = entry.response.clone();
                existing.created_unix_seconds = entry.created_unix_seconds;
            }
            existing.hit_count = existing.hit_count.max(entry.hit_count);
        } else {
            merged.entries.push(entry.clone());
        }
    }

    merged.entries.sort_by(|left, right| {
        right
            .created_unix_seconds
            .cmp(&left.created_unix_seconds)
            .then_with(|| right.hit_count.cmp(&left.hit_count))
    });
    merged.entries.truncate(MAX_QUERY_CACHE_ENTRIES);

    fs::write(
        &cache_path,
        serde_json::to_string_pretty(&merged).context("failed to serialize query cache")?,
    )
    .with_context(|| format!("failed to write {}", cache_path.display()))
}

fn render_summary(artifact: &GraphArtifact) -> String {
    let mut out = String::new();
    out.push_str("# HDC Code Graph Summary\n\n");
    out.push_str(&format!("Root: `{}`\n\n", artifact.root));
    out.push_str(&format!(
        "Files: {}  \nNodes: {}  \nEdges: {}  \nContext Packs: {}  \nCache Version: `{}`\n\n",
        artifact.summary.file_count,
        artifact.summary.node_count,
        artifact.summary.edge_count,
        artifact.context_packs.len(),
        artifact.cache_version
    ));

    out.push_str("## Relation Counts\n\n");
    for relation in &artifact.summary.relation_counts {
        out.push_str(&format!("- `{}`: {}\n", relation.name, relation.count));
    }
    out.push('\n');

    out.push_str("## Hottest Files\n\n");
    for file in &artifact.summary.hottest_files {
        out.push_str(&format!(
            "- `{}`: {} nodes, {} edges\n",
            file.path, file.node_count, file.edge_count
        ));
    }
    out.push('\n');

    out.push_str("## Strongest HDC File Pairs\n\n");
    for pair in &artifact.summary.strongest_pairs {
        out.push_str(&format!(
            "- `{}` <-> `{}`: {:.3}\n",
            pair.left, pair.right, pair.similarity
        ));
    }

    out
}

fn collect_source_files(root: &Path, current: &Path, output: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if file_type.is_dir() {
            if should_skip_dir(&name) {
                continue;
            }
            collect_source_files(root, &path, output)?;
        } else if file_type.is_file() {
            let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };

            if matches!(ext, "rs" | "wgsl") {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                output.push(rel);
            }
        }
    }
    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    matches!(name, ".git" | ".codex" | ".agents" | "target" | "graphify-out" | "node_modules")
}

struct InternalNode {
    record: NodeRecord,
    vector: HyperVector,
}

struct InternalEdge {
    record: EdgeRecord,
    vector: HyperVector,
}

struct UnresolvedCall {
    caller_id: String,
    callee_name: String,
    evidence_path: String,
    evidence_line: Option<usize>,
}

struct GraphBuilder {
    root: PathBuf,
    nodes: BTreeMap<String, InternalNode>,
    edges: Vec<InternalEdge>,
    file_node_ids: HashMap<String, String>,
    file_content_hashes: HashMap<String, String>,
    symbol_index: HashMap<String, BTreeSet<String>>,
    unresolved_calls: Vec<UnresolvedCall>,
}

impl GraphBuilder {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            file_node_ids: HashMap::new(),
            file_content_hashes: HashMap::new(),
            symbol_index: HashMap::new(),
            unresolved_calls: Vec::new(),
        }
    }

    fn record_file_content_hash(&mut self, rel_path: &str, content_hash: String) {
        self.file_content_hashes
            .insert(rel_path.to_string(), content_hash);
    }

    fn ensure_file_node(&mut self, rel_path: &str) -> String {
        if let Some(existing) = self.file_node_ids.get(rel_path) {
            return existing.clone();
        }

        let kind = if rel_path.ends_with(".wgsl") {
            "wgsl_file"
        } else {
            "rust_file"
        };
        let label = rel_path.rsplit('/').next().unwrap_or(rel_path).to_string();
        let node_id = self.insert_node(kind, label, rel_path.to_string(), None, vec![]);
        self.file_node_ids
            .insert(rel_path.to_string(), node_id.clone());
        node_id
    }

    fn ingest_rust(&mut self, rel_path: &str, file_node_id: &str, extracted: RustExtraction) {
        for node in extracted.nodes {
            let node_id = self.insert_node(
                node.kind,
                node.label.clone(),
                rel_path.to_string(),
                node.line,
                node.symbol_names,
            );
            self.push_edge(file_node_id, &node_id, "defines", rel_path, node.line);
        }

        for edge in extracted.edges {
            let source_id = self.lookup_or_insert_external(&edge.source_label);
            let target_id = self.lookup_or_insert_external(&edge.target_label);
            self.push_edge(&source_id, &target_id, edge.kind, rel_path, edge.line);
        }

        for raw_call in extracted.raw_calls {
            if let Some(caller_id) = self.lookup_symbol_exact(&raw_call.caller_label) {
                self.unresolved_calls.push(UnresolvedCall {
                    caller_id,
                    callee_name: raw_call.callee_name,
                    evidence_path: rel_path.to_string(),
                    evidence_line: raw_call.line,
                });
            }
        }

        for shader_ref in extracted.shader_refs {
            let target_id = self.ensure_file_node(&join_sibling(rel_path, &shader_ref.target_path));
            self.push_edge(
                file_node_id,
                &target_id,
                "loads_shader",
                rel_path,
                shader_ref.line,
            );
        }

        for pipeline in extracted.pipeline_refs {
            self.ingest_pipeline_reference(rel_path, file_node_id, pipeline);
        }
    }

    fn ingest_wgsl(&mut self, rel_path: &str, file_node_id: &str, extracted: WgslExtraction) {
        for node in extracted.nodes {
            let node_id = self.insert_node(
                node.kind,
                node.label.clone(),
                rel_path.to_string(),
                node.line,
                node.symbol_names,
            );
            self.push_edge(file_node_id, &node_id, "defines", rel_path, node.line);
        }

        for edge in extracted.edges {
            let source_id = self.lookup_or_insert_external(&edge.source_label);
            let target_id = self.lookup_or_insert_external(&edge.target_label);
            self.push_edge(&source_id, &target_id, edge.kind, rel_path, edge.line);
        }

        for raw_call in extracted.raw_calls {
            if let Some(caller_id) = self.lookup_symbol_exact(&raw_call.caller_label) {
                self.unresolved_calls.push(UnresolvedCall {
                    caller_id,
                    callee_name: raw_call.callee_name,
                    evidence_path: rel_path.to_string(),
                    evidence_line: raw_call.line,
                });
            }
        }
    }

    fn ingest_pipeline_reference(
        &mut self,
        rel_path: &str,
        file_node_id: &str,
        pipeline: PipelineReference,
    ) {
        let pipeline_id = self.insert_node(
            pipeline.kind,
            pipeline.label.clone(),
            rel_path.to_string(),
            pipeline.line,
            vec![pipeline.label.clone()],
        );
        self.push_edge(
            file_node_id,
            &pipeline_id,
            "defines",
            rel_path,
            pipeline.line,
        );

        let shader_rel = join_sibling(rel_path, &pipeline.shader_path);
        let shader_id = self.ensure_file_node(&shader_rel);
        self.push_edge(
            &pipeline_id,
            &shader_id,
            "uses_shader",
            rel_path,
            pipeline.line,
        );
    }

    fn finalize(mut self) -> Result<GraphArtifact> {
        let unresolved = std::mem::take(&mut self.unresolved_calls);
        for call in unresolved {
            let target_id = self.resolve_symbol(&call.callee_name);
            self.push_edge(
                &call.caller_id,
                &target_id,
                "calls",
                &call.evidence_path,
                call.evidence_line,
            );
        }

        let mut relation_counts: BTreeMap<String, usize> = BTreeMap::new();
        for edge in &self.edges {
            *relation_counts.entry(edge.record.kind.clone()).or_default() += 1;
        }

        let mut file_signatures = Vec::new();
        let mut rel_paths = self.file_node_ids.keys().cloned().collect::<Vec<_>>();
        rel_paths.sort();
        for rel_path in rel_paths {
            let node_vectors = self
                .nodes
                .values()
                .filter(|node| node.record.path == rel_path)
                .map(|node| node.vector)
                .collect::<Vec<_>>();

            let edge_vectors = self
                .edges
                .iter()
                .filter(|edge| edge.record.evidence_path == rel_path)
                .map(|edge| edge.vector)
                .collect::<Vec<_>>();

            let mut bundle = Vec::new();
            bundle.extend(node_vectors.iter().copied());
            bundle.extend(edge_vectors.iter().copied());

            let signature = HyperVector::bundle(&bundle);
            file_signatures.push(FileSignatureRecord {
                path: rel_path.clone(),
                kind: if rel_path.ends_with(".wgsl") {
                    "wgsl_file".to_string()
                } else {
                    "rust_file".to_string()
                },
                node_count: node_vectors.len(),
                edge_count: edge_vectors.len(),
                content_hash: self
                    .file_content_hashes
                    .get(&rel_path)
                    .cloned()
                    .unwrap_or_else(|| stable_hash_hex([CACHE_VERSION, "file", rel_path.as_str()])),
                signature: signature.to_hex(),
            });
        }

        file_signatures.sort_by(|left, right| left.path.cmp(&right.path));

        let mut incident_edges: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, edge) in self.edges.iter().enumerate() {
            incident_edges
                .entry(edge.record.source.clone())
                .or_default()
                .push(idx);
            incident_edges
                .entry(edge.record.target.clone())
                .or_default()
                .push(idx);
        }

        for node_id in self.nodes.keys().cloned().collect::<Vec<_>>() {
            let neighborhood_hash =
                compute_node_neighborhood_hash(&node_id, &self.nodes, &self.edges, &incident_edges);
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.record.neighborhood_hash = neighborhood_hash;
            }
        }

        let hottest_files = file_signatures
            .iter()
            .map(|file| FileHotspot {
                path: file.path.clone(),
                node_count: file.node_count,
                edge_count: file.edge_count,
            })
            .collect::<Vec<_>>();

        let mut hottest_files = hottest_files;
        hottest_files.sort_by(|left, right| {
            (right.node_count + right.edge_count).cmp(&(left.node_count + left.edge_count))
        });
        hottest_files.truncate(12);

        let mut strongest_pairs = Vec::new();
        for idx in 0..file_signatures.len() {
            for jdx in idx + 1..file_signatures.len() {
                let left = &file_signatures[idx];
                let right = &file_signatures[jdx];
                let Some(left_vec) = HyperVector::from_hex(&left.signature) else {
                    continue;
                };
                let Some(right_vec) = HyperVector::from_hex(&right.signature) else {
                    continue;
                };
                strongest_pairs.push(SimilarityPair {
                    left: left.path.clone(),
                    right: right.path.clone(),
                    similarity: left_vec.similarity(right_vec),
                });
            }
        }
        strongest_pairs.sort_by(|left, right| right.similarity.total_cmp(&left.similarity));
        strongest_pairs.truncate(20);

        let file_hash_by_path = file_signatures
            .iter()
            .map(|file| (file.path.clone(), file.content_hash.clone()))
            .collect::<HashMap<_, _>>();
        let context_packs =
            build_context_packs(&self.nodes, &self.edges, &incident_edges, &file_hash_by_path);

        let mut graph_parts = vec![CACHE_VERSION.to_string(), display_path(&self.root)];
        graph_parts.extend(
            file_signatures
                .iter()
                .map(|file| format!("file:{}:{}", file.path, file.content_hash)),
        );
        graph_parts.extend(self.nodes.values().map(|node| {
            format!(
                "node:{}:{}",
                node.record.stable_hash, node.record.neighborhood_hash
            )
        }));
        graph_parts.extend(
            self.edges
                .iter()
                .map(|edge| format!("edge:{}", edge.record.stable_hash)),
        );
        let graph_signature = stable_hash_hex(graph_parts.iter().map(|item| item.as_str()));

        let nodes = self
            .nodes
            .into_values()
            .map(|node| node.record)
            .collect::<Vec<_>>();
        let edges = self
            .edges
            .into_iter()
            .map(|edge| edge.record)
            .collect::<Vec<_>>();

        let generated_unix_seconds = unix_now();

        Ok(GraphArtifact {
            root: display_path(&self.root),
            generated_unix_seconds,
            cache_version: CACHE_VERSION.to_string(),
            summary: SummaryRecord {
                file_count: file_signatures.len(),
                node_count: nodes.len(),
                edge_count: edges.len(),
                relation_counts: relation_counts
                    .into_iter()
                    .map(|(name, count)| CountRecord { name, count })
                    .collect(),
                hottest_files,
                strongest_pairs,
            },
            nodes,
            edges,
            files: file_signatures,
            context_packs,
            graph_signature,
        })
    }

    fn insert_node(
        &mut self,
        kind: &str,
        label: String,
        path: String,
        line: Option<usize>,
        symbol_names: Vec<String>,
    ) -> String {
        let node_id = format!("{kind}|{path}|{}|{label}", line.unwrap_or(0));
        if !self.nodes.contains_key(&node_id) {
            let mut tokens = tokenize(&label);
            tokens.extend(tokenize(kind));
            tokens.extend(tokenize(&path));
            let vector = HyperVector::from_tokens(&tokens);
            let stable_hash = stable_hash_hex([
                CACHE_VERSION,
                "node",
                kind,
                path.as_str(),
                &line.unwrap_or(0).to_string(),
                label.as_str(),
            ]);
            let record = NodeRecord {
                id: node_id.clone(),
                kind: kind.to_string(),
                label: label.clone(),
                path: path.clone(),
                line,
                stable_hash,
                neighborhood_hash: String::new(),
                signature: vector.to_hex(),
            };

            self.nodes
                .insert(node_id.clone(), InternalNode { record, vector });
        }

        self.register_symbol(&label, &node_id);
        for symbol in symbol_names {
            self.register_symbol(&symbol, &node_id);
        }
        node_id
    }

    fn push_edge(
        &mut self,
        source_id: &str,
        target_id: &str,
        kind: &str,
        evidence_path: &str,
        evidence_line: Option<usize>,
    ) {
        let Some(source) = self.nodes.get(source_id) else {
            return;
        };
        let Some(target) = self.nodes.get(target_id) else {
            return;
        };

        let vector = HyperVector::relation(source.vector, kind, target.vector);
        let record = EdgeRecord {
            source: source_id.to_string(),
            target: target_id.to_string(),
            kind: kind.to_string(),
            evidence_path: evidence_path.to_string(),
            evidence_line,
            stable_hash: stable_hash_hex([
                CACHE_VERSION,
                "edge",
                source_id,
                target_id,
                kind,
                evidence_path,
                &evidence_line.unwrap_or(0).to_string(),
            ]),
            signature: vector.to_hex(),
        };
        self.edges.push(InternalEdge { record, vector });
    }

    fn register_symbol(&mut self, symbol: &str, node_id: &str) {
        let normalized = normalize_symbol(symbol);
        self.symbol_index
            .entry(normalized)
            .or_default()
            .insert(node_id.to_string());

        if let Some(short) = short_symbol(symbol) {
            self.symbol_index
                .entry(short)
                .or_default()
                .insert(node_id.to_string());
        }
    }

    fn lookup_symbol_exact(&self, symbol: &str) -> Option<String> {
        let normalized = normalize_symbol(symbol);
        self.symbol_index
            .get(&normalized)
            .and_then(|ids| pick_preferred_id(ids))
    }

    fn resolve_symbol(&mut self, symbol: &str) -> String {
        if let Some(found) = self.lookup_symbol_exact(symbol) {
            return found;
        }
        self.lookup_or_insert_external(symbol)
    }

    fn lookup_or_insert_external(&mut self, label: &str) -> String {
        if let Some(existing) = self.lookup_symbol_exact(label) {
            return existing;
        }
        self.insert_node(
            "external_symbol",
            label.to_string(),
            "<external>".to_string(),
            None,
            vec![label.to_string()],
        )
    }
}

fn compute_node_neighborhood_hash(
    node_id: &str,
    nodes: &BTreeMap<String, InternalNode>,
    edges: &[InternalEdge],
    incident_edges: &HashMap<String, Vec<usize>>,
) -> String {
    let mut parts = vec![CACHE_VERSION.to_string(), "neighborhood".to_string(), node_id.to_string()];
    let mut descriptors = incident_edges
        .get(node_id)
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(|idx| {
            let edge = edges.get(*idx)?;
            let other_id = if edge.record.source == node_id {
                edge.record.target.as_str()
            } else {
                edge.record.source.as_str()
            };
            let other = nodes.get(other_id)?;
            Some(format!(
                "{}|{}|{}|{}|{}",
                edge.record.kind,
                other.record.label,
                other.record.kind,
                edge.record.evidence_path,
                edge.record.evidence_line.unwrap_or(0)
            ))
        })
        .collect::<Vec<_>>();
    descriptors.sort();
    parts.extend(descriptors);
    stable_hash_hex(parts.iter().map(|item| item.as_str()))
}

fn build_context_packs(
    nodes: &BTreeMap<String, InternalNode>,
    edges: &[InternalEdge],
    incident_edges: &HashMap<String, Vec<usize>>,
    file_hash_by_path: &HashMap<String, String>,
) -> Vec<ContextPackRecord> {
    let mut packs = Vec::new();

    for (node_id, node) in nodes {
        if matches!(node.record.kind.as_str(), "rust_file" | "wgsl_file" | "external_symbol") {
            continue;
        }

        let Some(edge_ids) = incident_edges.get(node_id) else {
            continue;
        };

        let mut related = edge_ids
            .iter()
            .filter_map(|idx| {
                let edge = edges.get(*idx)?;
                let other_id = if edge.record.source == *node_id {
                    edge.record.target.clone()
                } else {
                    edge.record.source.clone()
                };
                let other = nodes.get(&other_id)?;
                Some((
                    edge.record.kind.clone(),
                    other.record.id.clone(),
                    other.record.label.clone(),
                    other.record.kind.clone(),
                    other.record.path.clone(),
                    other.record.line,
                ))
            })
            .collect::<Vec<_>>();

        related.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.4.cmp(&right.4))
        });

        let summary_lines = related
            .iter()
            .take(6)
            .map(|(edge_kind, _, other_label, other_kind, other_path, other_line)| {
                let location = match other_line {
                    Some(line) => format!("{other_path}:{line}"),
                    None => other_path.clone(),
                };
                format!("{edge_kind} -> {other_label} [{other_kind}] ({location})")
            })
            .collect::<Vec<_>>();

        let mut related_node_ids = related
            .iter()
            .map(|(_, other_id, _, _, _, _)| other_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        related_node_ids.truncate(10);

        let mut file_hashes = BTreeSet::new();
        if let Some(file_hash) = file_hash_by_path.get(&node.record.path) {
            file_hashes.insert(file_hash.clone());
        }
        for (_, _, _, _, other_path, _) in &related {
            if let Some(file_hash) = file_hash_by_path.get(other_path) {
                file_hashes.insert(file_hash.clone());
            }
        }
        let file_hashes = file_hashes.into_iter().collect::<Vec<_>>();

        let mut pack_parts = vec![
            CACHE_VERSION.to_string(),
            "pack".to_string(),
            node.record.stable_hash.clone(),
            node.record.neighborhood_hash.clone(),
        ];
        pack_parts.extend(file_hashes.iter().cloned());
        pack_parts.extend(related_node_ids.iter().cloned());
        let id = stable_hash_hex(pack_parts.iter().map(|item| item.as_str()));

        packs.push(ContextPackRecord {
            id,
            anchor_node_id: node.record.id.clone(),
            anchor_label: node.record.label.clone(),
            anchor_kind: node.record.kind.clone(),
            path: node.record.path.clone(),
            line: node.record.line,
            neighborhood_hash: node.record.neighborhood_hash.clone(),
            file_hashes,
            related_node_ids,
            summary_lines,
        });
    }

    packs.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.anchor_label.cmp(&right.anchor_label))
    });
    packs
}

fn pick_preferred_id(ids: &BTreeSet<String>) -> Option<String> {
    ids.iter()
        .find(|id| !id.starts_with("external_symbol|"))
        .cloned()
        .or_else(|| ids.iter().next().cloned())
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.replace(' ', "").to_ascii_lowercase()
}

fn short_symbol(symbol: &str) -> Option<String> {
    let compact = symbol.replace(' ', "");
    let short = compact
        .rsplit("::")
        .next()
        .filter(|item| !item.is_empty())
        .map(|item| item.to_ascii_lowercase());
    short
}

fn join_sibling(base_file: &str, child: &str) -> String {
    let parent = Path::new(base_file)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    normalize_rel_path(&parent.join(child).to_string_lossy().replace('\\', "/"))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string()
}

fn normalize_rel_path(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
