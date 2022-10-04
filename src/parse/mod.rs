pub mod tree;

mod expr;
mod matcher;
mod unconcretify;

use crate::hir::Expr;
use crate::lex::Token;
use crate::message::{File, Messages, Span};
use crate::Driver;
use matcher::Matcher;
use unconcretify::Unconcretifier;

pub fn parse(
    driver: &mut impl Driver,
    tokens: impl IntoIterator<Item = (Token, Span)>,
    file: File,
) -> Expr {
    let mut parser = Parser::new(tokens, file);
    let expr = parser.parse_expr();

    driver.report(parser.msgs);

    let mut unconcretifier = Unconcretifier::new();
    let expr = unconcretifier.unconcretify(expr);

    driver.report(unconcretifier.msgs);

    expr
}

#[derive(Debug)]
struct Parser<I> {
    tokens: I,
    curr: Option<(Token, Span)>,
    prev: Option<(Token, Span)>,
    msgs: Messages,
    default_span: Span,
}

impl<I> Parser<I>
where
    I: Iterator<Item = (Token, Span)>,
{
    pub fn new<In>(tokens: In, file: File) -> Self
    where
        In: IntoIterator<Item = (Token, Span), IntoIter = I>,
    {
        let mut parser = Self {
            tokens: tokens.into_iter(),

            curr: None,
            prev: None,

            msgs: Messages::new(),
            default_span: Span::new(file, 0, 0),
        };

        parser.advance();
        parser
    }

    fn is_done(&self) -> bool {
        self.curr.is_none()
    }

    fn advance(&mut self) {
        self.prev = self.curr.take();
        if let Some(curr) = self.tokens.next() {
            self.curr = Some(curr);
        }
    }

    fn peek(&self, matcher: impl Matcher) -> bool {
        self.curr
            .as_ref()
            .map(|(tok, _)| matcher.matches(tok))
            .unwrap_or(false)
    }

    fn consume(&mut self, matcher: impl Matcher) -> bool {
        if self.peek(matcher) {
            self.advance();
            true
        } else {
            false
        }
    }
}
