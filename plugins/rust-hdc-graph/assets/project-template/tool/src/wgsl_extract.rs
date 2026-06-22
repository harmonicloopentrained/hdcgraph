#[derive(Debug, Clone)]
pub struct ExtractedNode {
    pub kind: &'static str,
    pub label: String,
    pub line: Option<usize>,
    pub symbol_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractedEdge {
    pub source_label: String,
    pub target_label: String,
    pub kind: &'static str,
    pub line: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RawCall {
    pub caller_label: String,
    pub callee_name: String,
    pub line: Option<usize>,
}

#[derive(Debug, Default, Clone)]
pub struct WgslExtraction {
    pub nodes: Vec<ExtractedNode>,
    pub edges: Vec<ExtractedEdge>,
    pub raw_calls: Vec<RawCall>,
}

pub fn extract_wgsl(source: &str) -> WgslExtraction {
    let mut result = WgslExtraction::default();
    let lines: Vec<&str> = source.lines().collect();
    let mut functions: Vec<(String, usize)> = Vec::new();
    let mut current_stage: Option<&'static str> = None;

    for (idx, raw_line) in lines.iter().enumerate() {
        let line = raw_line.trim();

        if line.starts_with("@compute") {
            current_stage = Some("wgsl_entry_compute");
            continue;
        }
        if line.starts_with("@vertex") {
            current_stage = Some("wgsl_entry_vertex");
            continue;
        }
        if line.starts_with("@fragment") {
            current_stage = Some("wgsl_entry_fragment");
            continue;
        }

        if let Some(name) = parse_struct_name(line) {
            result.nodes.push(ExtractedNode {
                kind: "wgsl_struct",
                label: name.to_string(),
                line: Some(idx + 1),
                symbol_names: vec![name.to_string()],
            });
        }

        if let Some((group, binding, name)) = parse_binding_name(line) {
            let label = format!("binding[{group}:{binding}]::{name}");
            result.nodes.push(ExtractedNode {
                kind: "wgsl_binding",
                label,
                line: Some(idx + 1),
                symbol_names: vec![name.to_string()],
            });
        }

        if let Some(name) = parse_fn_name(line) {
            let kind = current_stage.take().unwrap_or("wgsl_function");
            result.nodes.push(ExtractedNode {
                kind,
                label: name.to_string(),
                line: Some(idx + 1),
                symbol_names: vec![name.to_string()],
            });
            functions.push((name.to_string(), idx));
        }
    }

    for (fn_name, start_idx) in &functions {
        let mut brace_depth = 0i32;
        let mut entered = false;
        for (idx, raw_line) in lines.iter().enumerate().skip(*start_idx) {
            let line = *raw_line;
            brace_depth += line.chars().filter(|ch| *ch == '{').count() as i32;
            if line.contains('{') {
                entered = true;
            }

            if entered {
                for (candidate, _) in &functions {
                    if candidate == fn_name {
                        continue;
                    }
                    if line.contains(&format!("{candidate}(")) && !line.contains("fn ") {
                        result.raw_calls.push(RawCall {
                            caller_label: fn_name.clone(),
                            callee_name: candidate.clone(),
                            line: Some(idx + 1),
                        });
                    }
                }
            }

            brace_depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
            if entered && brace_depth <= 0 {
                break;
            }
        }
    }

    result
}

fn parse_struct_name(line: &str) -> Option<&str> {
    if !line.starts_with("struct ") {
        return None;
    }
    let rest = &line["struct ".len()..];
    rest.split(|ch: char| ch == '{' || ch.is_whitespace())
        .next()
}

fn parse_fn_name(line: &str) -> Option<&str> {
    let marker = "fn ";
    let start = line.find(marker)?;
    let rest = &line[start + marker.len()..];
    rest.split('(')
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn parse_binding_name(line: &str) -> Option<(u32, u32, &str)> {
    if !line.contains("@group(") || !line.contains("@binding(") || !line.contains("var") {
        return None;
    }
    let group = parse_number_after(line, "@group(")?;
    let binding = parse_number_after(line, "@binding(")?;
    let var_idx = line.find("var")?;
    let rest = &line[var_idx..];
    let colon_idx = rest.find(':')?;
    let before_colon = rest[..colon_idx].trim();
    let name = before_colon.split_whitespace().last()?;
    Some((group, binding, name))
}

fn parse_number_after(line: &str, marker: &str) -> Option<u32> {
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find(')')?;
    rest[..end].trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::extract_wgsl;

    #[test]
    fn finds_entries_bindings_and_calls() {
        let source = r#"
struct SimParams {
  value: f32,
}

@group(0) @binding(0) var<storage, read> body_in: array<vec4f>;

fn helper() {
}

@compute @workgroup_size(8, 8, 1)
fn step_main() {
  helper();
}
"#;
        let extracted = extract_wgsl(source);
        assert!(extracted.nodes.iter().any(|node| node.label == "SimParams"));
        assert!(
            extracted
                .nodes
                .iter()
                .any(|node| node.kind == "wgsl_entry_compute" && node.label == "step_main")
        );
        assert_eq!(extracted.raw_calls.len(), 1);
    }
}
