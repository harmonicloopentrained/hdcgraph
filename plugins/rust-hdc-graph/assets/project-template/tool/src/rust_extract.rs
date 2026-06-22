use std::collections::HashMap;

use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    Expr, ExprMethodCall, File, FnArg, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemStruct,
    ItemTrait, ReturnType, Stmt, TraitItem, Type, spanned::Spanned,
};

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

#[derive(Debug, Clone)]
pub struct ShaderReference {
    pub target_path: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct PipelineReference {
    pub label: String,
    pub kind: &'static str,
    pub shader_path: String,
    pub line: Option<usize>,
}

#[derive(Debug, Default, Clone)]
pub struct RustExtraction {
    pub nodes: Vec<ExtractedNode>,
    pub edges: Vec<ExtractedEdge>,
    pub raw_calls: Vec<RawCall>,
    pub shader_refs: Vec<ShaderReference>,
    pub pipeline_refs: Vec<PipelineReference>,
}

pub fn extract_rust(relative_path: &str, source: &str) -> RustExtraction {
    let mut result = RustExtraction::default();
    result.shader_refs = scan_shader_refs(source);
    result.pipeline_refs = scan_pipeline_refs(source, &result.shader_refs);

    let Ok(file): Result<File, _> = syn::parse_file(source) else {
        return result;
    };

    let mut walker = RustWalker::new(relative_path);
    walker.walk_file(&file);
    result.nodes = walker.nodes;
    result.edges = walker.edges;
    result.raw_calls = walker.raw_calls;
    result
}

struct RustWalker<'a> {
    _path: &'a str,
    nodes: Vec<ExtractedNode>,
    edges: Vec<ExtractedEdge>,
    raw_calls: Vec<RawCall>,
}

impl<'a> RustWalker<'a> {
    fn new(path: &'a str) -> Self {
        Self {
            _path: path,
            nodes: Vec::new(),
            edges: Vec::new(),
            raw_calls: Vec::new(),
        }
    }

    fn walk_file(&mut self, file: &File) {
        for item in &file.items {
            self.walk_item(item);
        }
    }

    fn walk_item(&mut self, item: &Item) {
        match item {
            Item::Struct(item_struct) => self.handle_struct(item_struct),
            Item::Enum(item_enum) => self.handle_enum(item_enum),
            Item::Trait(item_trait) => self.handle_trait(item_trait),
            Item::Fn(item_fn) => self.handle_function(None, item_fn),
            Item::Impl(item_impl) => self.handle_impl(item_impl),
            _ => {}
        }
    }

    fn handle_struct(&mut self, item_struct: &ItemStruct) {
        let struct_name = item_struct.ident.to_string();
        self.nodes.push(ExtractedNode {
            kind: "rust_struct",
            label: struct_name.clone(),
            line: line_of(item_struct.ident.span()),
            symbol_names: vec![struct_name.clone()],
        });

        for field in &item_struct.fields {
            if let Some(ident) = &field.ident {
                let field_label = format!("{struct_name}.{}", ident);
                self.nodes.push(ExtractedNode {
                    kind: "rust_field",
                    label: field_label.clone(),
                    line: line_of(ident.span()),
                    symbol_names: vec![field_label.clone(), ident.to_string()],
                });
                self.edges.push(ExtractedEdge {
                    source_label: struct_name.clone(),
                    target_label: field_label,
                    kind: "has_field",
                    line: line_of(ident.span()),
                });

                let ty_label = normalize_type(&field.ty);
                self.edges.push(ExtractedEdge {
                    source_label: format!("{struct_name}.{}", ident),
                    target_label: ty_label,
                    kind: "typed_as",
                    line: line_of(field.ty.span()),
                });
            }
        }
    }

    fn handle_enum(&mut self, item_enum: &ItemEnum) {
        let enum_name = item_enum.ident.to_string();
        self.nodes.push(ExtractedNode {
            kind: "rust_enum",
            label: enum_name.clone(),
            line: line_of(item_enum.ident.span()),
            symbol_names: vec![enum_name.clone()],
        });

        for variant in &item_enum.variants {
            let variant_label = format!("{enum_name}::{}", variant.ident);
            self.nodes.push(ExtractedNode {
                kind: "rust_variant",
                label: variant_label.clone(),
                line: line_of(variant.ident.span()),
                symbol_names: vec![variant_label.clone(), variant.ident.to_string()],
            });
            self.edges.push(ExtractedEdge {
                source_label: enum_name.clone(),
                target_label: variant_label,
                kind: "has_variant",
                line: line_of(variant.ident.span()),
            });
        }
    }

    fn handle_trait(&mut self, item_trait: &ItemTrait) {
        let trait_name = item_trait.ident.to_string();
        self.nodes.push(ExtractedNode {
            kind: "rust_trait",
            label: trait_name.clone(),
            line: line_of(item_trait.ident.span()),
            symbol_names: vec![trait_name.clone()],
        });

        for trait_item in &item_trait.items {
            if let TraitItem::Fn(method) = trait_item {
                let label = format!("{trait_name}::{}", method.sig.ident);
                self.nodes.push(ExtractedNode {
                    kind: "rust_trait_method",
                    label: label.clone(),
                    line: line_of(method.sig.ident.span()),
                    symbol_names: vec![label.clone(), method.sig.ident.to_string()],
                });
                self.edges.push(ExtractedEdge {
                    source_label: trait_name.clone(),
                    target_label: label,
                    kind: "declares_method",
                    line: line_of(method.sig.ident.span()),
                });
            }
        }
    }

    fn handle_impl(&mut self, item_impl: &ItemImpl) {
        let self_ty = normalize_type(&item_impl.self_ty);
        if let Some((_, path, _)) = &item_impl.trait_ {
            let trait_name = path.to_token_stream().to_string().replace(' ', "");
            self.nodes.push(ExtractedNode {
                kind: "rust_trait",
                label: trait_name.clone(),
                line: line_of(path.span()),
                symbol_names: vec![trait_name.clone()],
            });
            self.edges.push(ExtractedEdge {
                source_label: self_ty.clone(),
                target_label: trait_name,
                kind: "implements",
                line: line_of(path.span()),
            });
        }

        for impl_item in &item_impl.items {
            if let ImplItem::Fn(method) = impl_item {
                let pseudo_fn = ItemFn {
                    attrs: Vec::new(),
                    vis: method.vis.clone(),
                    sig: method.sig.clone(),
                    block: Box::new(method.block.clone()),
                };
                self.handle_function(Some(&self_ty), &pseudo_fn);
            }
        }
    }

    fn handle_function(&mut self, owner: Option<&str>, item_fn: &ItemFn) {
        let fn_name = item_fn.sig.ident.to_string();
        let label = owner
            .map(|owner| format!("{owner}::{fn_name}"))
            .unwrap_or_else(|| fn_name.clone());

        self.nodes.push(ExtractedNode {
            kind: if owner.is_some() {
                "rust_method"
            } else {
                "rust_function"
            },
            label: label.clone(),
            line: line_of(item_fn.sig.ident.span()),
            symbol_names: vec![label.clone(), fn_name.clone()],
        });

        if let Some(owner) = owner {
            self.edges.push(ExtractedEdge {
                source_label: owner.to_string(),
                target_label: label.clone(),
                kind: "owns_method",
                line: line_of(item_fn.sig.ident.span()),
            });
        }

        for input in &item_fn.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                let ty_label = normalize_type(&pat_type.ty);
                self.edges.push(ExtractedEdge {
                    source_label: label.clone(),
                    target_label: ty_label,
                    kind: "accepts_type",
                    line: line_of(pat_type.ty.span()),
                });
            }
        }

        if let ReturnType::Type(_, ty) = &item_fn.sig.output {
            self.edges.push(ExtractedEdge {
                source_label: label.clone(),
                target_label: normalize_type(ty),
                kind: "returns_type",
                line: line_of(ty.span()),
            });
        }

        extract_calls_from_block(&item_fn.block.stmts, &label, &mut self.raw_calls);
    }
}

fn extract_calls_from_block(stmts: &[Stmt], caller_label: &str, output: &mut Vec<RawCall>) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _) => extract_calls_from_expr(expr, caller_label, output),
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    extract_calls_from_expr(&init.expr, caller_label, output);
                }
            }
            _ => {}
        }
    }
}

fn extract_calls_from_expr(expr: &Expr, caller_label: &str, output: &mut Vec<RawCall>) {
    match expr {
        Expr::Call(call_expr) => {
            let callee = expr_name(&call_expr.func);
            if !callee.is_empty() {
                output.push(RawCall {
                    caller_label: caller_label.to_string(),
                    callee_name: callee,
                    line: line_of(call_expr.func.span()),
                });
            }
            for arg in &call_expr.args {
                extract_calls_from_expr(arg, caller_label, output);
            }
        }
        Expr::MethodCall(call) => extract_method_call(call, caller_label, output),
        Expr::Block(expr_block) => {
            extract_calls_from_block(&expr_block.block.stmts, caller_label, output)
        }
        Expr::If(expr_if) => {
            extract_calls_from_expr(&expr_if.cond, caller_label, output);
            extract_calls_from_block(&expr_if.then_branch.stmts, caller_label, output);
            if let Some((_, else_expr)) = &expr_if.else_branch {
                extract_calls_from_expr(else_expr, caller_label, output);
            }
        }
        Expr::Match(expr_match) => {
            extract_calls_from_expr(&expr_match.expr, caller_label, output);
            for arm in &expr_match.arms {
                extract_calls_from_expr(&arm.body, caller_label, output);
                if let Some((_, guard)) = &arm.guard {
                    extract_calls_from_expr(guard, caller_label, output);
                }
            }
        }
        Expr::While(expr_while) => {
            extract_calls_from_expr(&expr_while.cond, caller_label, output);
            extract_calls_from_block(&expr_while.body.stmts, caller_label, output);
        }
        Expr::ForLoop(expr_for) => {
            extract_calls_from_expr(&expr_for.expr, caller_label, output);
            extract_calls_from_block(&expr_for.body.stmts, caller_label, output);
        }
        Expr::Loop(expr_loop) => {
            extract_calls_from_block(&expr_loop.body.stmts, caller_label, output)
        }
        Expr::Binary(expr_binary) => {
            extract_calls_from_expr(&expr_binary.left, caller_label, output);
            extract_calls_from_expr(&expr_binary.right, caller_label, output);
        }
        Expr::Unary(expr_unary) => extract_calls_from_expr(&expr_unary.expr, caller_label, output),
        Expr::Assign(expr_assign) => {
            extract_calls_from_expr(&expr_assign.left, caller_label, output);
            extract_calls_from_expr(&expr_assign.right, caller_label, output);
        }
        Expr::Field(expr_field) => extract_calls_from_expr(&expr_field.base, caller_label, output),
        Expr::Reference(expr_ref) => extract_calls_from_expr(&expr_ref.expr, caller_label, output),
        Expr::Paren(expr_paren) => extract_calls_from_expr(&expr_paren.expr, caller_label, output),
        Expr::Closure(expr_closure) => {
            extract_calls_from_expr(&expr_closure.body, caller_label, output)
        }
        Expr::Array(expr_array) => {
            for elem in &expr_array.elems {
                extract_calls_from_expr(elem, caller_label, output);
            }
        }
        Expr::Tuple(expr_tuple) => {
            for elem in &expr_tuple.elems {
                extract_calls_from_expr(elem, caller_label, output);
            }
        }
        _ => {}
    }
}

fn extract_method_call(call: &ExprMethodCall, caller_label: &str, output: &mut Vec<RawCall>) {
    output.push(RawCall {
        caller_label: caller_label.to_string(),
        callee_name: call.method.to_string(),
        line: line_of(call.method.span()),
    });
    extract_calls_from_expr(&call.receiver, caller_label, output);
    for arg in &call.args {
        extract_calls_from_expr(arg, caller_label, output);
    }
}

fn expr_name(expr: &Expr) -> String {
    match expr {
        Expr::Path(path) => path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        Expr::Field(field) => field.member.to_token_stream().to_string(),
        Expr::Paren(inner) => expr_name(&inner.expr),
        _ => String::new(),
    }
}

fn normalize_type(ty: &Type) -> String {
    ty.to_token_stream().to_string().replace(' ', "")
}

fn line_of(span: Span) -> Option<usize> {
    let line = span.start().line;
    if line == 0 { None } else { Some(line) }
}

fn scan_shader_refs(source: &str) -> Vec<ShaderReference> {
    let mut refs = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let mut cursor = line;
        while let Some(start) = cursor.find("include_str!(\"") {
            let rest = &cursor[start + "include_str!(\"".len()..];
            if let Some(end) = rest.find("\")") {
                refs.push(ShaderReference {
                    target_path: rest[..end].to_string(),
                    line: Some(idx + 1),
                });
                cursor = &rest[end + 2..];
            } else {
                break;
            }
        }
    }
    refs
}

fn scan_pipeline_refs(source: &str, shader_refs: &[ShaderReference]) -> Vec<PipelineReference> {
    let mut result = Vec::new();
    let mut shader_var_map: HashMap<String, String> = HashMap::new();
    let shader_paths: Vec<String> = shader_refs
        .iter()
        .map(|item| item.target_path.clone())
        .collect();

    let lines: Vec<&str> = source.lines().collect();
    let mut idx = 0usize;
    while idx < lines.len() {
        let line = lines[idx].trim();
        if let Some(var) = parse_let_var(line) {
            let is_shader_module = line.contains("create_shader_module")
                || line.contains("create_embedded_shader_module")
                || line.contains("create_owned_shader_module");
            let is_pipeline = line.contains("create_compute_pipeline")
                || line.contains("create_render_pipeline")
                || line.contains("register_compute")
                || line.contains("register_render");

            if is_shader_module {
                let (body, end_idx, start_line) = collect_statement(&lines, idx);
                if let Some(shader_path) = infer_shader_path(&body, &shader_paths) {
                    shader_var_map.insert(var.to_string(), shader_path);
                }
                idx = end_idx;
                if idx + 1 < start_line {
                    idx += 1;
                }
            } else if is_pipeline {
                let (body, end_idx, start_line) = collect_statement(&lines, idx);
                let kind = if line.contains("compute_pipeline") || line.contains("register_compute")
                {
                    "wgpu_compute_pipeline"
                } else {
                    "wgpu_render_pipeline"
                };
                let label = infer_pipeline_label(&body).unwrap_or_else(|| var.to_string());

                if let Some(shader_path) = infer_pipeline_shader(&body, &shader_var_map) {
                    result.push(PipelineReference {
                        label,
                        kind,
                        shader_path,
                        line: Some(start_line),
                    });
                }
                idx = end_idx;
                if idx + 1 < start_line {
                    idx += 1;
                }
            }
        } else if line.contains(": pipelines.register_compute(")
            || line.contains(": pipelines.register_render(")
        {
            let field_name = line
                .split(':')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            let (body, end_idx, start_line) = collect_statement(&lines, idx);
            let kind = if line.contains("register_compute") {
                "wgpu_compute_pipeline"
            } else {
                "wgpu_render_pipeline"
            };
            let label = infer_pipeline_label(&body).unwrap_or(field_name);

            if let Some(shader_path) = infer_pipeline_shader(&body, &shader_var_map) {
                result.push(PipelineReference {
                    label,
                    kind,
                    shader_path,
                    line: Some(start_line),
                });
            }
            idx = end_idx;
        }
        idx += 1;
    }
    result
}

fn parse_let_var(line: &str) -> Option<&str> {
    if !line.starts_with("let ") {
        return None;
    }
    Some(
        line["let ".len()..]
            .split('=')
            .next()
            .unwrap_or_default()
            .trim(),
    )
}

fn collect_statement(lines: &[&str], start_idx: usize) -> (String, usize, usize) {
    let mut body = String::new();
    let mut idx = start_idx;
    let mut paren_depth = 0i32;
    let start_line = start_idx + 1;

    while idx < lines.len() {
        let line = lines[idx];
        body.push_str(line);
        body.push('\n');
        for ch in line.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                _ => {}
            }
        }

        if paren_depth <= 0
            && (line.contains(");")
                || line.contains("),")
                || line.trim_end().ends_with("))")
                || line.trim_end().ends_with("}),")
                || line.trim_end().ends_with("})"))
        {
            break;
        }
        idx += 1;
    }

    (body, idx, start_line)
}

fn infer_shader_path(body: &str, shader_paths: &[String]) -> Option<String> {
    for shader_path in shader_paths {
        if body.contains(&format!("include_str!(\"{shader_path}\")")) {
            return Some(shader_path.clone());
        }
    }

    let bundle_mappings = [
        ("chrysalis_compute_shader_bundle()", "../../compute.wgsl"),
        ("chrysalis_fluid_shader_bundle()", "../../fluid.wgsl"),
        (
            "chrysalis_octree_stream_shader_bundle()",
            "../../octree_stream.wgsl",
        ),
        (
            "chrysalis_occupancy_shader_bundle()",
            "../../occupancy.wgsl",
        ),
        ("chrysalis_present_shader_bundle()", "../../present.wgsl"),
        ("chrysalis_render_shader_bundle()", "../../render.wgsl"),
    ];
    for (needle, shader_path) in bundle_mappings {
        if body.contains(needle) {
            return Some(shader_path.to_string());
        }
    }

    let embedded_mappings = [
        ("BOOT_COMPUTE_WGSL", "embedded/boot_compute.wgsl"),
        (
            "BOOT_OCTREE_STREAM_WGSL",
            "embedded/boot_octree_stream.wgsl",
        ),
        ("BOOT_OCCUPANCY_WGSL", "embedded/boot_occupancy.wgsl"),
        ("BOOT_RENDER_WGSL", "embedded/boot_render.wgsl"),
    ];
    for (needle, shader_path) in embedded_mappings {
        if body.contains(needle) {
            return Some(shader_path.to_string());
        }
    }

    None
}

fn infer_pipeline_label(body: &str) -> Option<String> {
    if let Some(label_start) = body.find("label: Some(\"") {
        let rest = &body[label_start + "label: Some(\"".len()..];
        if let Some(end) = rest.find("\")") {
            return Some(rest[..end].to_string());
        }
    }
    extract_first_string_literal(body)
}

fn infer_pipeline_shader(body: &str, shader_var_map: &HashMap<String, String>) -> Option<String> {
    for (module_var, shader_path) in shader_var_map {
        if body.contains(&format!("module: &{module_var}"))
            || body.contains(&format!("&{module_var},"))
            || body.contains(&format!("&{module_var}\n"))
        {
            return Some(shader_path.clone());
        }
    }
    None
}

fn extract_first_string_literal(body: &str) -> Option<String> {
    let start = body.find('"')?;
    let rest = &body[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::extract_rust;

    #[test]
    fn finds_shader_refs_and_pipelines() {
        let source = r#"
let compute_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
});
let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("compute pipeline"),
    module: &compute_module,
});
"#;
        let extracted = extract_rust("src/main.rs", source);
        assert_eq!(extracted.shader_refs.len(), 1);
        assert_eq!(extracted.pipeline_refs.len(), 1);
        assert_eq!(extracted.pipeline_refs[0].shader_path, "compute.wgsl");
    }

    #[test]
    fn finds_helper_registered_pipelines() {
        let source = r#"
let compute_module = create_owned_shader_module(
    resources,
    "chrysalis full compute wgsl",
    assemble_chrysalis_shader(chrysalis_compute_shader_bundle()),
);

let compute_pipeline = pipelines.register_compute(ComputePipelineDesc::single_layout(
    "5D body compute pipeline",
    "compute pipeline layout",
    &compute_module,
    compute_bind_group_layout,
));

ChrysalisFullComputePipelines {
    raw_chart_smooth_pipeline: pipelines.register_compute(ComputePipelineDesc::single_layout_entry(
        "raw chart devoxelize bake pipeline",
        "compute pipeline layout",
        &compute_module,
        compute_bind_group_layout,
        "raw_chart_smooth_main",
    )),
}
"#;
        let extracted = extract_rust("src/modules/chrysalis/pipelines.rs", source);
        assert_eq!(extracted.pipeline_refs.len(), 2);
        assert!(
            extracted
                .pipeline_refs
                .iter()
                .all(|item| item.shader_path == "../../compute.wgsl")
        );
    }
}
