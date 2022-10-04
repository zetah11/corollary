use crate::mir::{Decls, Expr, ExprNode, Pat, PatNode, Type, TypeId, Types, ValueDef};
use crate::tyck;

type HiType = tyck::Type;
type HiPat = tyck::Pat<HiType>;
type HiPatNode = tyck::PatNode;
type HiExpr = tyck::Expr<HiType>;
type HiExprNode = tyck::ExprNode<HiType>;
type HiDecls = tyck::Decls<HiType>;

pub fn lower(decls: HiDecls) -> (Types, Decls) {
    let mut types = Types::new();
    let decls = lower_decls(&mut types, decls);
    (types, decls)
}

fn lower_decls(types: &mut Types, decls: HiDecls) -> Decls {
    let mut values = Vec::with_capacity(decls.values.len());

    for def in decls.values {
        let pat = lower_pat(types, def.pat);
        let bind = lower_expr(types, def.bind);

        values.push(ValueDef {
            span: def.span,
            pat,
            bind,
        });
    }

    Decls { values }
}

fn lower_expr(types: &mut Types, ex: HiExpr) -> Expr {
    if let HiType::Invalid = ex.data {
        Expr {
            node: ExprNode::Invalid,
            span: ex.span,
            typ: types.add(Type::Invalid),
        }
    } else {
        let node = match ex.node {
            HiExprNode::Int(v) => ExprNode::Int(v),

            HiExprNode::Name(name) => ExprNode::Name(name),

            HiExprNode::Lam(param, body) => {
                let param = lower_pat(types, param);
                let body = lower_expr(types, *body);
                ExprNode::Lam(param, Box::new(body))
            }

            HiExprNode::App(fun, arg) => {
                let fun = lower_expr(types, *fun);
                let arg = lower_expr(types, *arg);

                ExprNode::App(Box::new(fun), Box::new(arg))
            }

            HiExprNode::Invalid => ExprNode::Invalid,

            HiExprNode::Anno(..) => unreachable!(),
        };

        Expr {
            node,
            span: ex.span,
            typ: lower_type(types, ex.data),
        }
    }
}

fn lower_pat(types: &mut Types, pat: HiPat) -> Pat {
    let node = match pat.node {
        HiPatNode::Name(name) => PatNode::Name(name),
        HiPatNode::Invalid => PatNode::Invalid,
    };

    Pat {
        node,
        span: pat.span,
        typ: lower_type(types, pat.data),
    }
}

fn lower_type(types: &mut Types, ty: HiType) -> TypeId {
    match ty {
        HiType::Fun(t, u) => {
            let t = lower_type(types, *t);
            let u = lower_type(types, *u);

            types.add(Type::Fun(t, u))
        }

        HiType::Range(lo, hi) => types.add(Type::Range(lo, hi)),

        HiType::Invalid | HiType::Number => unreachable!(),
    }
}
