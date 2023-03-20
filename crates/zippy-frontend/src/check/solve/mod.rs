mod flow;
mod instantiate;
mod types;

use std::collections::HashMap;

use zippy_common::messages::{Message, MessageMaker};
use zippy_common::source::Span;

use crate::messages::TypeMessages;
use crate::Db;

use self::types::NumericResult;
use super::types::{CoercionState, CoercionVar, Coercions, Constraint, Template, Type, UnifyVar};

#[derive(Debug)]
pub struct Solution {
    pub messages: Vec<Message>,
    pub substitution: HashMap<UnifyVar, Type>,
}

pub fn solve(db: &dyn Db, counts: HashMap<Span, usize>, constraints: Vec<Constraint>) -> Solution {
    let mut solver = Solver::new(db, counts, constraints);
    solver.solve();

    Solution {
        messages: solver.messages,
        substitution: solver.substitution,
    }
}

struct Solver<'db> {
    db: &'db dyn Db,
    messages: Vec<Message>,
    counts: HashMap<Span, usize>,

    constraints: Vec<Constraint>,
    type_numeric: Vec<(Span, Type)>,

    coercions: Coercions,
    substitution: HashMap<UnifyVar, Type>,
}

impl<'db> Solver<'db> {
    pub fn new(
        db: &'db dyn Db,
        counts: HashMap<Span, usize>,
        constraints: Vec<Constraint>,
    ) -> Self {
        Self {
            db,
            messages: Vec::new(),
            counts,

            constraints,
            type_numeric: Vec::new(),

            coercions: Coercions::new(),
            substitution: HashMap::new(),
        }
    }

    pub fn solve(&mut self) {
        while !self.constraints.is_empty() {
            let constraints: Vec<_> = self.constraints.drain(..).collect();
            let before = constraints.len();

            for constraint in constraints {
                self.solve_constraint(constraint);
            }

            let after = self.constraints.len();

            if after >= before {
                let constraints: Vec<_> = self.constraints.drain(..).collect();
                for constraint in constraints {
                    self.report_unsolvable(constraint);
                }
                break;
            }
        }

        self.solve_type_numerics();
    }

    fn solve_constraint(&mut self, constraint: Constraint) {
        match constraint {
            Constraint::Assignable { at, id, into, from } => self.assign(at, id, into, from),

            Constraint::Equal(at, t, u) => self.equate(at, t, u),

            Constraint::Field {
                at,
                target,
                of,
                field,
            } => self.field(at, target, of, field),

            Constraint::Instantiated(at, ty, template) => self.instantiated(at, ty, template),

            Constraint::UnitLike(at, ty) => self.unitlike(at, ty),
            Constraint::Numeric(at, ty) => match self.numeric(at, ty) {
                NumericResult::Ok => {}
                NumericResult::Unsolved(at, ty) => {
                    self.constraints.push(Constraint::Numeric(at, ty))
                }
                NumericResult::Error(messages) => self.messages.extend(messages),
            },

            Constraint::Textual(at, ty) => self.textual(at, ty),
            Constraint::TypeNumeric(at, ty) => self.type_numeric.push((at, ty)),
        }
    }

    fn solve_type_numerics(&mut self) {
        let constraints: Vec<_> = self.type_numeric.drain(..).collect();
        for (at, ty) in constraints {
            match self.numeric(at, ty) {
                NumericResult::Ok => {}
                NumericResult::Unsolved(at, ty) => {
                    self.equate(at, ty, Type::Number);
                }

                NumericResult::Error(messages) => self.messages.extend(messages),
            }
        }
    }

    /// Create a unique type.
    fn fresh(&mut self, span: Span) -> Type {
        let counter = self.counts.entry(span).or_insert(0);
        let count = *counter;
        *counter += 1;

        Type::Var(UnifyVar { span, count })
    }

    /// Report an unsolvable constraint.
    fn report_unsolvable(&mut self, constraint: Constraint) {
        let span = match constraint {
            Constraint::Assignable { at, .. } => at,
            Constraint::Equal(at, _, _) => at,
            Constraint::Field { at, .. } => at,
            Constraint::Instantiated(at, _, _) => at,
            Constraint::UnitLike(at, _) => at,
            Constraint::Numeric(at, _) => at,
            Constraint::Textual(at, _) => at,
            Constraint::TypeNumeric(at, _) => at,
        };

        self.at(span).ambiguous();
    }

    fn at(&mut self, span: Span) -> MessageMaker<&mut Vec<Message>> {
        MessageMaker::new(&mut self.messages, span)
    }

    fn common_db(&self) -> &'db dyn zippy_common::Db {
        <dyn Db as salsa::DbWithJar<zippy_common::Jar>>::as_jar_db(self.db)
    }
}
