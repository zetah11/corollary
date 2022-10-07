use super::Resolver;
use crate::hir::{Expr, ExprNode};

impl Resolver {
    pub fn declare_expr(&mut self, expr: &Expr) {
        match &expr.node {
            ExprNode::Name(_) | ExprNode::Int(_) | ExprNode::Hole | ExprNode::Invalid => {}
            ExprNode::Lam(id, param, body) => {
                self.enter(param.span, *id);
                self.declare_pat(param);
                self.declare_expr(body);
                self.exit();
            }

            ExprNode::App(fun, arg) => {
                self.declare_expr(fun);
                self.declare_expr(arg);
            }

            ExprNode::Anno(expr, _) => self.declare_expr(expr),
        }
    }
}
