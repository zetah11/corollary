use crate::message::Span;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BindId(pub(crate) usize);

#[derive(Debug)]
pub struct Expr<Name = String> {
    pub node: ExprNode<Name>,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprNode<Name> {
    Name(Name),
    Int(i64),

    Lam(BindId, Pat<Name>, Box<Expr<Name>>),
    App(Box<Expr<Name>>, Box<Expr<Name>>),

    Anno(Box<Expr<Name>>, Type),

    Invalid,
}

#[derive(Debug)]
pub struct Pat<Name = String> {
    pub node: PatNode<Name>,
    pub span: Span,
}

#[derive(Debug)]
pub enum PatNode<Name> {
    Name(Name),
    Invalid,
}

#[derive(Debug)]
pub struct Type {
    pub node: TypeNode,
    pub span: Span,
}

#[derive(Debug)]
pub enum TypeNode {
    Range(i64, i64),
    Fun(Box<Type>, Box<Type>),
    Invalid,
}
