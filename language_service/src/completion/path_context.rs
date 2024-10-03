// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use qsc::{
    ast::{
        visit::{self, Visitor},
        Attr, Block, CallableDecl, Expr, ExprKind, FieldAssign, FieldDef, FunctorExpr, Ident, Item,
        ItemKind, Namespace, Package, Pat, Path, PathResult, QubitInit, SpecDecl, Stmt, StructDecl,
        Ty, TyDef, TyKind,
    },
    parse::completion::PathKind,
};
use std::rc::Rc;

/// Provides the qualifier and the expected name kind for the
/// incomplete path (e.g. `foo.bar.`) at the cursor offset.
///
/// Methods may panic if the offset does not fall within an incomplete path.
#[derive(Debug)]
pub(super) struct IncompletePath<'a> {
    qualifier: Option<&'a [Ident]>,
    context: Option<PathKind>,
    offset: u32,
}

impl<'a> IncompletePath<'a> {
    pub fn init(offset: u32, package: &'a Package) -> Self {
        let mut offset_visitor = OffsetVisitor {
            offset,
            visitor: IncompletePath {
                offset,
                context: None,
                qualifier: None,
            },
        };

        offset_visitor.visit_package(package);

        offset_visitor.visitor
    }
}

impl<'a> Visitor<'a> for IncompletePath<'a> {
    fn visit_item(&mut self, item: &Item) {
        match *item.kind {
            ItemKind::Open(..) => self.context = Some(PathKind::Namespace),
            ItemKind::ImportOrExport(..) => self.context = Some(PathKind::Import),
            _ => {}
        }
    }

    fn visit_ty(&mut self, ty: &Ty) {
        if let TyKind::Path(..) = *ty.kind {
            self.context = Some(PathKind::Ty);
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if let ExprKind::Path(..) = *expr.kind {
            self.context = Some(PathKind::Expr);
        } else if let ExprKind::Struct(..) = *expr.kind {
            self.context = Some(PathKind::Struct);
        }
    }

    fn visit_path_result(&mut self, path: &'a PathResult) {
        self.qualifier = match path {
            PathResult::Ok(path) => path.segments.as_ref().map(AsRef::as_ref),
            PathResult::Err(Some(incomplete_path)) => Some(&incomplete_path.segments),
            PathResult::Err(None) => None,
        };
    }
}

impl IncompletePath<'_> {
    pub fn context(&self) -> (PathKind, Vec<Rc<str>>) {
        let qualifier = self.segments_before_offset();

        // WARNING: this assumption appears to hold true today, but it's subtle
        // enough that parser and AST changes can easily violate it in the future.
        assert!(
            !qualifier.is_empty(),
            "path segment completion should only be invoked for a partially parsed path"
        );

        let context = self
            .context
            .expect("context must exist for path segment completion");

        (context, qualifier)
    }

    fn segments_before_offset(&self) -> Vec<Rc<str>> {
        self.qualifier
            .into_iter()
            .flat_map(AsRef::as_ref)
            .take_while(|i| i.span.hi < self.offset)
            .map(|i| i.name.clone())
            .collect::<Vec<_>>()
    }
}

/// A [`Visitor`] wrapper that only descends into a node
/// if the given offset falls within that node.
struct OffsetVisitor<T> {
    offset: u32,
    visitor: T,
}

impl<'a, T> Visitor<'a> for OffsetVisitor<T>
where
    T: Visitor<'a>,
{
    fn visit_namespace(&mut self, namespace: &'a Namespace) {
        if namespace.span.touches(self.offset) {
            self.visitor.visit_namespace(namespace);
            visit::walk_namespace(self, namespace);
        }
    }

    fn visit_item(&mut self, item: &'a Item) {
        if item.span.touches(self.offset) {
            self.visitor.visit_item(item);
            visit::walk_item(self, item);
        }
    }

    fn visit_attr(&mut self, attr: &'a Attr) {
        if attr.span.touches(self.offset) {
            self.visitor.visit_attr(attr);
            visit::walk_attr(self, attr);
        }
    }

    fn visit_ty_def(&mut self, def: &'a TyDef) {
        if def.span.touches(self.offset) {
            self.visitor.visit_ty_def(def);
            visit::walk_ty_def(self, def);
        }
    }

    fn visit_callable_decl(&mut self, decl: &'a CallableDecl) {
        if decl.span.touches(self.offset) {
            self.visitor.visit_callable_decl(decl);
            visit::walk_callable_decl(self, decl);
        }
    }

    fn visit_struct_decl(&mut self, decl: &'a StructDecl) {
        if decl.span.touches(self.offset) {
            self.visitor.visit_struct_decl(decl);
            visit::walk_struct_decl(self, decl);
        }
    }

    fn visit_field_def(&mut self, def: &'a FieldDef) {
        if def.span.touches(self.offset) {
            self.visitor.visit_field_def(def);
            visit::walk_field_def(self, def);
        }
    }

    fn visit_spec_decl(&mut self, decl: &'a SpecDecl) {
        if decl.span.touches(self.offset) {
            self.visitor.visit_spec_decl(decl);
            visit::walk_spec_decl(self, decl);
        }
    }

    fn visit_functor_expr(&mut self, expr: &'a FunctorExpr) {
        if expr.span.touches(self.offset) {
            self.visitor.visit_functor_expr(expr);
            visit::walk_functor_expr(self, expr);
        }
    }

    fn visit_ty(&mut self, ty: &'a Ty) {
        if ty.span.touches(self.offset) {
            self.visitor.visit_ty(ty);
            visit::walk_ty(self, ty);
        }
    }

    fn visit_block(&mut self, block: &'a Block) {
        if block.span.touches(self.offset) {
            self.visitor.visit_block(block);
            visit::walk_block(self, block);
        }
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if stmt.span.touches(self.offset) {
            self.visitor.visit_stmt(stmt);
            visit::walk_stmt(self, stmt);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if expr.span.touches(self.offset) {
            self.visitor.visit_expr(expr);
            visit::walk_expr(self, expr);
        }
    }

    fn visit_field_assign(&mut self, assign: &'a FieldAssign) {
        if assign.span.touches(self.offset) {
            self.visitor.visit_field_assign(assign);
            visit::walk_field_assign(self, assign);
        }
    }

    fn visit_pat(&mut self, pat: &'a Pat) {
        if pat.span.touches(self.offset) {
            self.visitor.visit_pat(pat);
            visit::walk_pat(self, pat);
        }
    }

    fn visit_qubit_init(&mut self, init: &'a QubitInit) {
        if init.span.touches(self.offset) {
            self.visitor.visit_qubit_init(init);
            visit::walk_qubit_init(self, init);
        }
    }

    fn visit_path(&mut self, path: &'a Path) {
        if path.span.touches(self.offset) {
            self.visitor.visit_path(path);
            visit::walk_path(self, path);
        }
    }

    fn visit_path_result(&mut self, path: &'a PathResult) {
        let span = match path {
            PathResult::Ok(path) => &path.span,
            PathResult::Err(Some(incomplete_path)) => &incomplete_path.span,
            PathResult::Err(None) => return,
        };

        if span.touches(self.offset) {
            self.visitor.visit_path_result(path);
            visit::walk_path_result(self, path);
        }
    }

    fn visit_ident(&mut self, ident: &'a Ident) {
        if ident.span.touches(self.offset) {
            self.visitor.visit_ident(ident);
        }
    }
}
