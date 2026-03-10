use std::collections::{HashMap, HashSet};
use std::mem;
use std::sync::Arc;
use swc_core::common::{
    DUMMY_SP, FileName, FilePathMapping, GLOBALS, Mark, SourceMap, Span, Spanned, SyntaxContext,
};
use swc_core::ecma::ast::{
    ArrayLit, ArrowExpr, AssignExpr, AwaitExpr, BinExpr, BinaryOp, CallExpr, Callee, CondExpr,
    EsVersion, Expr, ExprOrSpread, FnDecl, FnExpr, ForStmt, Ident, IdentName, IfStmt, ImportDecl,
    ImportNamedSpecifier, ImportPhase, ImportSpecifier, Lit, MemberExpr, MemberProp, ModuleDecl,
    ModuleItem, NewExpr, ObjectLit, ParenExpr, Pass, Pat, Program, Prop, PropOrSpread, SeqExpr,
    Str, TaggedTpl, UnaryExpr, UpdateExpr, VarDeclarator, WhileStmt, YieldExpr,
};
use swc_core::ecma::codegen::{Config, Emitter, text_writer::JsWriter};
use swc_core::ecma::parser::{Parser, StringInput, Syntax};
use swc_core::ecma::transforms::testing::test_inline;
// use swc_core::ecma::transforms::typescript::strip_type;
use swc_core::ecma::transforms::typescript::{Config as TsConfig, typescript};
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExprId(Span);

impl From<Span> for ExprId {
    fn from(span: Span) -> Self {
        ExprId(span)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprContext {
    TernaryCondition,
    TernaryConsequent,
    TernaryAlternate,
    LogicalAndLeft,
    LogicalAndRight,
    LogicalOrLeft,
    LogicalOrRight,
    NullishLeft,
    NullishRight,
    BinaryLeft,
    BinaryRight,
    UnaryOperand,
    UpdateOperand,
    AssignRight,
    IfCondition,
    WhileCondition,
    ForCondition,
    ForUpdate,
    ArrayElement(usize),
    ObjectValue,
    ObjectSpread,
    SequenceItem(usize),
    YieldValue,
    AwaitValue,
    TaggedTemplate,
    Parenthesized,
    FunctionCall,
    NewExpression,
    Root,
}

#[derive(Default)]
pub struct AsyncCollector {
    async_functions: HashSet<String>,
}

impl AsyncCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

impl VisitMut for AsyncCollector {
    fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
        if node.function.is_async {
            self.async_functions
                .insert(node.ident.sym.as_str().to_string());
        }
        node.visit_mut_children_with(self);
    }

    fn visit_mut_fn_expr(&mut self, node: &mut FnExpr) {
        if node.function.is_async
            && let Some(ident) = &node.ident
        {
            self.async_functions.insert(ident.sym.as_str().to_string());
        }
        node.visit_mut_children_with(self);
    }

    fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
        node.visit_mut_children_with(self);
    }

    fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
        if let Some(init) = &node.init {
            match init.as_ref() {
                Expr::Arrow(arrow) if arrow.is_async => {
                    if let Pat::Ident(ident) = &node.name {
                        self.async_functions.insert(ident.sym.as_str().to_string());
                    }
                }
                Expr::Fn(fn_expr) if fn_expr.function.is_async => {
                    if let Pat::Ident(ident) = &node.name {
                        self.async_functions.insert(ident.sym.as_str().to_string());
                    }
                }
                _ => {}
            }
        }
        node.visit_mut_children_with(self);
    }
}

pub struct ContextAnalyzer {
    expression_contexts: HashMap<ExprId, ExprContext>,
    function_calls: HashSet<ExprId>,
    current_context: ExprContext,
}

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextAnalyzer {
    pub fn new() -> Self {
        Self {
            expression_contexts: HashMap::new(),
            function_calls: HashSet::new(),
            current_context: ExprContext::Root,
        }
    }

    fn with_context<F>(&mut self, context: ExprContext, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let old_context = mem::replace(&mut self.current_context, context);
        f(self);
        self.current_context = old_context;
    }

    fn mark_expression(&mut self, expr: &Expr, context: ExprContext) {
        let expr_id = ExprId::from(expr.span());
        self.expression_contexts.insert(expr_id, context);
    }

    fn mark_function_call(&mut self, expr: &CallExpr) {
        let expr_id = ExprId::from(expr.span);
        self.function_calls.insert(expr_id);
        self.mark_expression(&Expr::Call(expr.clone()), ExprContext::FunctionCall);
    }
}

impl VisitMut for ContextAnalyzer {
    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        self.mark_function_call(node);
        node.visit_mut_children_with(self);
    }

    fn visit_mut_new_expr(&mut self, node: &mut NewExpr) {
        let expr_id = ExprId::from(node.span);
        self.function_calls.insert(expr_id);
        self.mark_expression(&Expr::New(node.clone()), ExprContext::NewExpression);
        node.visit_mut_children_with(self);
    }

    fn visit_mut_cond_expr(&mut self, node: &mut CondExpr) {
        self.with_context(ExprContext::TernaryCondition, |analyzer| {
            analyzer.mark_expression(&node.test, ExprContext::TernaryCondition);
            node.test.visit_mut_with(analyzer);
        });
        self.with_context(ExprContext::TernaryConsequent, |analyzer| {
            analyzer.mark_expression(&node.cons, ExprContext::TernaryConsequent);
            node.cons.visit_mut_with(analyzer);
        });
        self.with_context(ExprContext::TernaryAlternate, |analyzer| {
            analyzer.mark_expression(&node.alt, ExprContext::TernaryAlternate);
            node.alt.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_bin_expr(&mut self, node: &mut BinExpr) {
        let (left_context, right_context) = match node.op {
            BinaryOp::LogicalAnd => (ExprContext::LogicalAndLeft, ExprContext::LogicalAndRight),
            BinaryOp::LogicalOr => (ExprContext::LogicalOrLeft, ExprContext::LogicalOrRight),
            BinaryOp::NullishCoalescing => (ExprContext::NullishLeft, ExprContext::NullishRight),
            _ => (ExprContext::BinaryLeft, ExprContext::BinaryRight),
        };

        self.with_context(left_context.clone(), |analyzer| {
            analyzer.mark_expression(&node.left, left_context);
            node.left.visit_mut_with(analyzer);
        });
        self.with_context(right_context.clone(), |analyzer| {
            analyzer.mark_expression(&node.right, right_context);
            node.right.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_unary_expr(&mut self, node: &mut UnaryExpr) {
        self.with_context(ExprContext::UnaryOperand, |analyzer| {
            analyzer.mark_expression(&node.arg, ExprContext::UnaryOperand);
            node.arg.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_update_expr(&mut self, node: &mut UpdateExpr) {
        self.with_context(ExprContext::UpdateOperand, |analyzer| {
            analyzer.mark_expression(&node.arg, ExprContext::UpdateOperand);
            node.arg.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_assign_expr(&mut self, node: &mut AssignExpr) {
        node.left.visit_mut_with(self);
        self.with_context(ExprContext::AssignRight, |analyzer| {
            analyzer.mark_expression(&node.right, ExprContext::AssignRight);
            node.right.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_seq_expr(&mut self, node: &mut SeqExpr) {
        for (i, expr) in node.exprs.iter_mut().enumerate() {
            self.with_context(ExprContext::SequenceItem(i), |analyzer| {
                analyzer.mark_expression(expr, ExprContext::SequenceItem(i));
                expr.visit_mut_with(analyzer);
            });
        }
    }

    fn visit_mut_yield_expr(&mut self, node: &mut YieldExpr) {
        if let Some(arg) = &mut node.arg {
            self.with_context(ExprContext::YieldValue, |analyzer| {
                analyzer.mark_expression(arg, ExprContext::YieldValue);
                arg.visit_mut_with(analyzer);
            });
        }
    }

    fn visit_mut_await_expr(&mut self, node: &mut AwaitExpr) {
        self.with_context(ExprContext::AwaitValue, |analyzer| {
            analyzer.mark_expression(&node.arg, ExprContext::AwaitValue);
            node.arg.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_tagged_tpl(&mut self, node: &mut TaggedTpl) {
        self.with_context(ExprContext::TaggedTemplate, |analyzer| {
            analyzer.mark_expression(&node.tag, ExprContext::TaggedTemplate);
            node.tag.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_paren_expr(&mut self, node: &mut ParenExpr) {
        self.with_context(ExprContext::Parenthesized, |analyzer| {
            analyzer.mark_expression(&node.expr, ExprContext::Parenthesized);
            node.expr.visit_mut_with(analyzer);
        });
    }

    fn visit_mut_array_lit(&mut self, node: &mut ArrayLit) {
        for (i, elem) in node.elems.iter_mut().enumerate() {
            if let Some(expr_or_spread) = elem {
                self.with_context(ExprContext::ArrayElement(i), |analyzer| {
                    analyzer.mark_expression(&expr_or_spread.expr, ExprContext::ArrayElement(i));
                    expr_or_spread.expr.visit_mut_with(analyzer);
                });
            }
        }
    }

    fn visit_mut_object_lit(&mut self, node: &mut ObjectLit) {
        for prop in &mut node.props {
            match prop {
                PropOrSpread::Spread(spread) => {
                    self.with_context(ExprContext::ObjectSpread, |analyzer| {
                        analyzer.mark_expression(&spread.expr, ExprContext::ObjectSpread);
                        spread.expr.visit_mut_with(analyzer);
                    });
                }
                PropOrSpread::Prop(prop) => match prop.as_mut() {
                    Prop::KeyValue(kv) => {
                        self.with_context(ExprContext::ObjectValue, |analyzer| {
                            analyzer.mark_expression(&kv.value, ExprContext::ObjectValue);
                            kv.value.visit_mut_with(analyzer);
                        });
                    }
                    Prop::Getter(getter) => {
                        if let Some(body) = &mut getter.body {
                            body.visit_mut_children_with(self);
                        }
                    }
                    Prop::Setter(setter) => {
                        if let Some(body) = &mut setter.body {
                            body.visit_mut_children_with(self);
                        }
                    }
                    Prop::Method(method) => {
                        method.function.visit_mut_children_with(self);
                    }
                    _ => {}
                },
            }
        }
    }

    fn visit_mut_if_stmt(&mut self, node: &mut IfStmt) {
        self.with_context(ExprContext::IfCondition, |analyzer| {
            analyzer.mark_expression(&node.test, ExprContext::IfCondition);
            node.test.visit_mut_with(analyzer);
        });
        node.cons.visit_mut_with(self);
        if let Some(alt) = &mut node.alt {
            alt.visit_mut_with(self);
        }
    }

    fn visit_mut_while_stmt(&mut self, node: &mut WhileStmt) {
        self.with_context(ExprContext::WhileCondition, |analyzer| {
            analyzer.mark_expression(&node.test, ExprContext::WhileCondition);
            node.test.visit_mut_with(analyzer);
        });
        node.body.visit_mut_with(self);
    }

    fn visit_mut_for_stmt(&mut self, node: &mut ForStmt) {
        if let Some(init) = &mut node.init {
            init.visit_mut_with(self);
        }
        if let Some(test) = &mut node.test {
            self.with_context(ExprContext::ForCondition, |analyzer| {
                analyzer.mark_expression(test, ExprContext::ForCondition);
                test.visit_mut_with(analyzer);
            });
        }
        if let Some(update) = &mut node.update {
            self.with_context(ExprContext::ForUpdate, |analyzer| {
                analyzer.mark_expression(update, ExprContext::ForUpdate);
                update.visit_mut_with(analyzer);
            });
        }
        node.body.visit_mut_with(self);
    }
}

pub struct DualPhaseTransformer {
    async_functions: HashSet<String>,
    expression_contexts: HashMap<ExprId, ExprContext>,
    function_calls: HashSet<ExprId>,
}

impl DualPhaseTransformer {
    pub fn new(async_functions: HashSet<String>) -> Self {
        Self {
            async_functions,
            expression_contexts: HashMap::new(),
            function_calls: HashSet::new(),
        }
    }

    pub fn set_analysis_results(
        &mut self,
        contexts: HashMap<ExprId, ExprContext>,
        function_calls: HashSet<ExprId>,
    ) {
        self.expression_contexts = contexts;
        self.function_calls = function_calls;
    }

    fn is_method_chaining(&self, member_expr: &MemberExpr) -> bool {
        match &member_expr.obj.as_ref() {
            Expr::Call(_) => true,
            Expr::Member(nested) => self.is_method_chaining(nested),
            _ => false,
        }
    }

    fn is_checkpoint_reference(&self, member_expr: &MemberExpr) -> bool {
        match &member_expr.obj.as_ref() {
            Expr::Ident(ident) => ident.sym.as_str() == "__checkpoint__",
            Expr::Member(nested) => self.is_checkpoint_reference(nested),
            _ => false,
        }
    }

    fn is_complex_computed_access(&self, member_expr: &MemberExpr) -> bool {
        matches!(member_expr.prop, MemberProp::Computed(_))
    }

    fn should_skip_transformation(&self, member_expr: &MemberExpr) -> bool {
        if self.is_checkpoint_reference(member_expr) {
            return true;
        }
        if self.is_method_chaining(member_expr) {
            return true;
        }
        if self.is_complex_computed_access(member_expr) {
            return true;
        }
        false
    }

    fn extract_call_info(&self, node: &CallExpr) -> (Option<String>, Option<Expr>) {
        match &node.callee {
            Callee::Expr(expr) => match expr.as_ref() {
                Expr::Ident(ident) => (Some(ident.sym.as_str().to_string()), None),
                Expr::Member(member_expr) => {
                    if self.should_skip_transformation(member_expr) {
                        return (None, None);
                    }
                    let name = extract_member_name(member_expr);
                    let context = extract_member_context(member_expr);
                    (Some(name), Some(context))
                }
                _ => (None, None),
            },
            _ => (None, None),
        }
    }
}

impl VisitMut for DualPhaseTransformer {
    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        node.visit_mut_children_with(self);
        let expr_id = ExprId::from(node.span);
        if self.function_calls.contains(&expr_id) {
            let (function_name, this_context) = self.extract_call_info(node);
            if let Some(name) = function_name {
                let is_async = self.async_functions.contains(&name);
                let wrapper = if is_async {
                    create_async_wrapper(&name, &node.args, this_context)
                } else {
                    create_sync_wrapper(&name, &node.args, this_context)
                };
                *node = wrapper;
            }
        }
    }

    fn visit_mut_new_expr(&mut self, node: &mut NewExpr) {
        node.visit_mut_children_with(self);
    }

    fn visit_mut_program(&mut self, program: &mut Program) {
        let named_specifier = ImportNamedSpecifier {
            span: DUMMY_SP,
            local: Ident::new_no_ctxt("__checkpoint__".into(), DUMMY_SP),
            imported: None,
            is_type_only: false,
        };
        let import_specifier = ImportSpecifier::Named(named_specifier);
        let import_decl = ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![import_specifier],
            src: Box::new(Str {
                span: DUMMY_SP,
                value: "../runtime/checkpoint-runtime".into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: ImportPhase::Evaluation,
        };
        let module_item = ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl));
        match program {
            Program::Module(module) => {
                module.body.insert(0, module_item);
            }
            Program::Script(_) => {}
        }
        program.visit_mut_children_with(self);
    }
}

impl Pass for DualPhaseTransformer {
    fn process(&mut self, program: &mut Program) {
        program.visit_mut_with(self);
    }
}

#[derive(Debug)]
pub enum TransformError {
    ParseError(String),
    TransformError(String),
    CodegenError(String),
}

fn create_async_wrapper(name: &str, args: &[ExprOrSpread], context: Option<Expr>) -> CallExpr {
    let checkpoint_ident = Ident::new_no_ctxt("__checkpoint__".into(), DUMMY_SP);
    let execute_async_ident = IdentName::new("executeAsync".into(), DUMMY_SP);
    let member_expr = MemberExpr {
        obj: Box::new(Expr::Ident(checkpoint_ident)),
        prop: MemberProp::Ident(execute_async_ident),
        span: DUMMY_SP,
    };
    let function_name_lit = Lit::Str(Str {
        value: name.into(),
        raw: None,
        span: DUMMY_SP,
    });
    let function_ident = Ident::new_no_ctxt(name.into(), DUMMY_SP);
    let function_name_arg = ExprOrSpread {
        expr: Box::new(Expr::Lit(function_name_lit)),
        spread: None,
    };
    let function_identifier_arg = ExprOrSpread {
        expr: Box::new(Expr::Ident(function_ident)),
        spread: None,
    };
    let original_args_array = ArrayLit {
        span: DUMMY_SP,
        elems: args.iter().map(|arg| Some(arg.clone())).collect(),
    };
    let original_args_arg = ExprOrSpread {
        expr: Box::new(Expr::Array(original_args_array)),
        spread: None,
    };
    let mut call_args = vec![
        function_name_arg,
        function_identifier_arg,
        original_args_arg,
    ];
    if let Some(ctx) = context {
        let context_arg = ExprOrSpread {
            expr: Box::new(ctx),
            spread: None,
        };
        call_args.push(context_arg);
    }
    CallExpr {
        callee: Callee::Expr(Box::new(Expr::Member(member_expr))),
        args: call_args,
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        type_args: None,
    }
}

fn create_sync_wrapper(name: &str, args: &[ExprOrSpread], context: Option<Expr>) -> CallExpr {
    let checkpoint_ident = Ident::new_no_ctxt("__checkpoint__".into(), DUMMY_SP);
    let execute_ident = IdentName::new("execute".into(), DUMMY_SP);
    let member_expr = MemberExpr {
        obj: Box::new(Expr::Ident(checkpoint_ident)),
        prop: MemberProp::Ident(execute_ident),
        span: DUMMY_SP,
    };
    let function_name_lit = Lit::Str(Str {
        value: name.into(),
        raw: None,
        span: DUMMY_SP,
    });
    let function_ident = Ident::new_no_ctxt(name.into(), DUMMY_SP);
    let function_name_arg = ExprOrSpread {
        expr: Box::new(Expr::Lit(function_name_lit)),
        spread: None,
    };
    let function_identifier_arg = ExprOrSpread {
        expr: Box::new(Expr::Ident(function_ident)),
        spread: None,
    };
    let original_args_array = ArrayLit {
        span: DUMMY_SP,
        elems: args.iter().map(|arg| Some(arg.clone())).collect(),
    };
    let original_args_arg = ExprOrSpread {
        expr: Box::new(Expr::Array(original_args_array)),
        spread: None,
    };
    let mut call_args = vec![
        function_name_arg,
        function_identifier_arg,
        original_args_arg,
    ];
    if let Some(ctx) = context {
        let context_arg = ExprOrSpread {
            expr: Box::new(ctx),
            spread: None,
        };
        call_args.push(context_arg);
    }
    CallExpr {
        callee: Callee::Expr(Box::new(Expr::Member(member_expr))),
        args: call_args,
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        type_args: None,
    }
}

pub fn transform_code(
    source: &str,
    file_path: &str,
    minify: bool,
) -> Result<String, TransformError> {
    GLOBALS.set(&Default::default(), || {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();
        let syntax = if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
            Syntax::Typescript(Default::default())
        } else {
            Syntax::Es(Default::default())
        };
        let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
        let fm = cm.new_source_file(FileName::Real(file_path.into()).into(), source.to_string());
        let input = StringInput::from(&*fm);
        let mut parser = Parser::new(syntax, input, None);
        let module = parser
            .parse_module()
            .map_err(|e| TransformError::ParseError(format!("{:?}", e)))?;
        let mut program = Program::Module(module);
        if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
            program.mutate(typescript(
                TsConfig::default(),
                unresolved_mark,
                top_level_mark,
            ))
        }
        let mut collector = AsyncCollector::new();
        program.visit_mut_with(&mut collector);
        let mut analyzer = ContextAnalyzer::new();
        program.visit_mut_with(&mut analyzer);
        let mut transformer = DualPhaseTransformer::new(collector.async_functions);
        transformer.set_analysis_results(analyzer.expression_contexts, analyzer.function_calls);
        program.visit_mut_with(&mut transformer);
        let mut buf = Vec::new();
        let wr = JsWriter::new(cm.clone(), "\n", &mut buf, None);
        let mut emitter = Emitter {
            cfg: Config::default()
                .with_minify(minify)
                .with_omit_last_semi(true)
                .with_target(EsVersion::Es2024),
            cm: cm.clone(),
            comments: None,
            wr,
        };
        emitter
            .emit_program(&program)
            .map_err(|e| TransformError::CodegenError(format!("{:?}", e)))?;
        String::from_utf8(buf).map_err(|e| TransformError::CodegenError(format!("{:?}", e)))
    })
}

fn extract_member_name(member_expr: &MemberExpr) -> String {
    let obj_name = match &member_expr.obj.as_ref() {
        Expr::Ident(ident) => ident.sym.as_str().to_string(),
        Expr::Member(nested_member) => extract_member_name(nested_member),
        Expr::This(_) => "this".to_string(),
        _ => "unknown".to_string(),
    };
    let prop_name = match &member_expr.prop {
        MemberProp::Ident(ident) => ident.sym.as_str().to_string(),
        MemberProp::Computed(_) => "computed".to_string(),
        _ => "unknown".to_string(),
    };
    format!("{}.{}", obj_name, prop_name)
}

fn extract_member_context(member_expr: &MemberExpr) -> Expr {
    match &member_expr.obj.as_ref() {
        Expr::Member(nested_member) => Expr::Member(nested_member.clone()),
        Expr::This(this_expr) => Expr::This(*this_expr),
        _ => (*member_expr.obj).clone(),
    }
}

#[allow(dead_code)]
fn create_test_transformer(source: &str, async_functions: HashSet<String>) -> DualPhaseTransformer {
    let syntax = Syntax::Es(Default::default());
    let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
    let fm = cm.new_source_file(FileName::Real("test.js".into()).into(), source.to_string());
    let input = StringInput::from(&*fm);
    let mut parser = Parser::new(syntax, input, None);
    let module = parser.parse_module().unwrap();
    let mut test_program = Program::Module(module);
    let mut collector = AsyncCollector::new();
    collector.async_functions = async_functions;
    let mut analyzer = ContextAnalyzer::new();
    test_program.visit_mut_with(&mut analyzer);
    let mut transformer = DualPhaseTransformer::new(collector.async_functions);
    transformer.set_analysis_results(analyzer.expression_contexts, analyzer.function_calls);
    transformer
}

test_inline!(
    Default::default(),
    |_| {
        let mut async_funcs = HashSet::new();
        async_funcs.insert("fetchData".to_string());
        create_test_transformer(
            r#"export {}; async function fetchData() { return "data"; } fetchData()"#,
            async_funcs,
        )
    },
    async_function_call,
    r#"export {}; async function fetchData() { return "data"; } fetchData()"#,
    r#"import { __checkpoint__ } from "../runtime/checkpoint-runtime";
export {};
async function fetchData() { return "data"; }
__checkpoint__.executeAsync("fetchData", fetchData, []);"#
);
