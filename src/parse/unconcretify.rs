//! The parser itself is fairly accepting, not distinguishing between patterns, types, or expressions; for instance
//! `(x => x) => x` gives a valid parse. The job of this module is to produce a HIR tree, validating away cases like
//! that in the process.

use super::tree as cst;
use crate::hir::{self, BindId};
use crate::message::Messages;

#[derive(Debug, Default)]
pub struct Unconcretifier {
    pub msgs: Messages,
    bind_id: usize,
}

impl Unconcretifier {
    pub fn new() -> Self {
        Self {
            msgs: Messages::new(),
            bind_id: 0,
        }
    }

    pub fn unconcretify(&mut self, expr: cst::Expr) -> hir::Expr {
        self.unconc_expr(expr)
    }

    fn unconc_expr(&mut self, expr: cst::Expr) -> hir::Expr {
        let node = match expr.node {
            cst::ExprNode::Name(name) => hir::ExprNode::Name(name),
            cst::ExprNode::Int(i) => hir::ExprNode::Int(i as i64), // uhhh
            cst::ExprNode::Group(expr) => return self.unconc_expr(*expr),
            cst::ExprNode::Range(span, lo, hi) => {
                let lo = Box::new(self.unconc_expr(*lo));
                let hi = Box::new(self.unconc_expr(*hi));
                let fun = hir::Expr {
                    node: hir::ExprNode::Name("upto".into()),
                    span,
                };

                let span = lo.span + span;
                let fun = hir::Expr {
                    node: hir::ExprNode::App(Box::new(fun), lo),
                    span,
                };

                hir::ExprNode::App(Box::new(fun), hi)
            }
            cst::ExprNode::Fun(span, t, u) => {
                let t = Box::new(self.unconc_expr(*t));
                let u = Box::new(self.unconc_expr(*u));

                let fun = hir::Expr {
                    node: hir::ExprNode::Name("->".into()),
                    span,
                };

                let span = t.span + span;
                let fun = hir::Expr {
                    node: hir::ExprNode::App(Box::new(fun), t),
                    span,
                };

                hir::ExprNode::App(Box::new(fun), u)
            }
            cst::ExprNode::Lam(pat, body) => {
                let pat = self.unconc_pat(*pat);
                let body = Box::new(self.unconc_expr(*body));
                hir::ExprNode::Lam(self.fresh_bind_id(), pat, body)
            }
            cst::ExprNode::App(fun, arg) => {
                let fun = Box::new(self.unconc_expr(*fun));
                let arg = Box::new(self.unconc_expr(*arg));
                hir::ExprNode::App(fun, arg)
            }
            cst::ExprNode::Anno(expr, anno) => {
                let expr = Box::new(self.unconc_expr(*expr));
                let anno = self.unconc_type(*anno);
                hir::ExprNode::Anno(expr, anno)
            }
            cst::ExprNode::Invalid => hir::ExprNode::Invalid,
        };

        hir::Expr {
            node,
            span: expr.span,
        }
    }

    fn unconc_pat(&mut self, pat: cst::Expr) -> hir::Pat {
        let node = match pat.node {
            cst::ExprNode::Name(name) => hir::PatNode::Name(name),
            cst::ExprNode::Group(pat) => return self.unconc_pat(*pat),
            cst::ExprNode::Invalid => hir::PatNode::Invalid,
            _ => {
                self.msgs.at(pat.span).parse_not_a_pattern();
                hir::PatNode::Invalid
            }
        };

        hir::Pat {
            node,
            span: pat.span,
        }
    }

    fn unconc_type(&mut self, typ: cst::Expr) -> hir::Type {
        let node = match typ.node {
            cst::ExprNode::Range(_, lo, hi) => {
                let lo = self.unconc_expr(*lo);
                let hi = self.unconc_expr(*hi);

                match (lo.node, hi.node) {
                    (hir::ExprNode::Int(lo), hir::ExprNode::Int(hi)) => {
                        hir::TypeNode::Range(lo, hi)
                    }

                    (hir::ExprNode::Int(_), _) => {
                        self.msgs.at(hi.span).parse_range_not_an_int();
                        hir::TypeNode::Invalid
                    }

                    (_, hir::ExprNode::Int(_)) => {
                        self.msgs.at(lo.span).parse_range_not_an_int();
                        hir::TypeNode::Invalid
                    }

                    _ => {
                        self.msgs.at(lo.span + hi.span).parse_range_not_an_int();
                        hir::TypeNode::Invalid
                    }
                }
            }

            cst::ExprNode::Fun(_, t, u) => {
                let t = Box::new(self.unconc_type(*t));
                let u = Box::new(self.unconc_type(*u));
                hir::TypeNode::Fun(t, u)
            }

            cst::ExprNode::Group(typ) => return self.unconc_type(*typ),

            _ => {
                self.msgs.at(typ.span).parse_not_a_type();
                hir::TypeNode::Invalid
            }
        };

        hir::Type {
            node,
            span: typ.span,
        }
    }

    fn fresh_bind_id(&mut self) -> BindId {
        let id = BindId(self.bind_id);
        self.bind_id += 1;
        id
    }
}
