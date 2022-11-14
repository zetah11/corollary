use std::collections::HashMap;

use common::message::{Messages, Span};
use common::names::{Name, Names};
use common::thir::{merge_insts, pretty_type, Because, Mutability, Type, UniVar};

#[derive(Debug)]
pub struct Unifier<'a> {
    pub names: &'a Names,

    pub subst: HashMap<UniVar, (HashMap<Name, Type>, Type)>,
    pub causes: HashMap<UniVar, Because>,
    pub worklist: Vec<(Span, Type, Type)>,
    pub messages: Messages,
}

impl<'a> Unifier<'a> {
    pub fn new(names: &'a Names) -> Self {
        Self {
            names,
            subst: HashMap::new(),
            causes: HashMap::new(),
            worklist: Vec::new(),
            messages: Messages::new(),
        }
    }

    pub fn unify(&mut self, span: Span, expected: Type, actual: Type) {
        self.unify_within(&HashMap::new(), span, expected, actual)
    }

    fn unify_within(
        &mut self,
        inst: &HashMap<Name, Type>,
        span: Span,
        expected: Type,
        actual: Type,
    ) {
        match (expected, actual) {
            (Type::Range(lo1, hi1), Type::Range(lo2, hi2)) => {
                if !(lo1 <= lo2 && hi1 >= hi2) {
                    self.messages
                        .at(span)
                        .tyck_narrow_range((lo1, hi1), (lo2, hi2));
                }
            }

            (Type::Number, Type::Range(..)) => {}

            (Type::Name(n), u) if inst.contains_key(&n) => {
                let t = inst.get(&n).unwrap();
                self.unify_within(inst, span, t.clone(), u)
            }

            (t, Type::Name(m)) if inst.contains_key(&m) => {
                let u = inst.get(&m).unwrap();
                self.unify_within(inst, span, t, u.clone())
            }

            (Type::Name(n), Type::Name(m)) => {
                if n != m {
                    let n: Option<String> = None;
                    let m: Option<String> = None;
                    self.messages.at(span).tyck_incompatible(n, m);
                }
            }

            (Type::Fun(t1, u1), Type::Fun(t2, u2)) => {
                self.unify_within(inst, span, *t1, *t2);
                self.unify_within(inst, span, *u1, *u2);
            }

            (Type::Product(t1, u1), Type::Product(t2, u2)) => {
                self.unify_within(inst, span, *t1, *t2);
                self.unify_within(inst, span, *u1, *u2);
            }

            (Type::Var(_, v), Type::Var(_, w)) if v == w => {}

            (Type::Var(mutability, v), u) => {
                if let Some((other_inst, t)) = self.get(mutability, &v) {
                    let inst = merge_insts(inst, other_inst);
                    return self.unify_within(&inst, span, t, u);
                }

                if mutability == Mutability::Mutable {
                    if Self::occurs(&v, &u) {
                        self.messages
                            .at(span)
                            .tyck_recursive_inference(format!("{v:?}"), format!("{u:?}"));
                        self.set(inst, v, Type::Invalid);
                    } else {
                        self.set(inst, v, u);
                        self.causes.insert(v, Because::Unified(span));
                    }
                } else if inst.is_empty() {
                    self.worklist.push((span, Type::Var(mutability, v), u));
                } else {
                    self.worklist.push((
                        span,
                        Type::Instantiated(Box::new(Type::Var(mutability, v)), inst.clone()),
                        u,
                    ));
                }
            }

            (t, Type::Var(mutability, w)) => {
                if let Some((other_inst, u)) = self.get(mutability, &w) {
                    let inst = merge_insts(inst, other_inst);
                    return self.unify_within(&inst, span, t, u);
                }

                if mutability == Mutability::Mutable {
                    if Self::occurs(&w, &t) {
                        self.messages
                            .at(span)
                            .tyck_recursive_inference(format!("{w:?}"), format!("{t:?}"));
                        self.set(inst, w, Type::Invalid);
                    } else {
                        self.set(inst, w, t);
                        self.causes.insert(w, Because::Unified(span));
                    }
                } else if inst.is_empty() {
                    self.worklist.push((span, t, Type::Var(mutability, w)));
                } else {
                    self.worklist.push((
                        span,
                        t,
                        Type::Instantiated(Box::new(Type::Var(mutability, w)), inst.clone()),
                    ));
                }
            }

            (Type::Instantiated(t, other_inst), u) => {
                let new: HashMap<_, _> = inst
                    .iter()
                    .chain(other_inst.iter())
                    .map(|(name, var)| (*name, var.clone()))
                    .collect();

                self.unify_within(&new, span, *t, u)
            }

            (t, Type::Instantiated(u, other_inst)) => {
                let new: HashMap<_, _> = inst
                    .iter()
                    .chain(other_inst.iter())
                    .map(|(name, var)| (*name, var.clone()))
                    .collect();

                self.unify_within(&new, span, t, *u)
            }

            (Type::Invalid, _) | (_, Type::Invalid) => {}

            (expected, actual) => {
                let subst = self.subst.iter().map(|(var, (_, ty))| (*var, ty)).collect();
                let expected = pretty_type(self.names, &subst, &expected);
                let actual = pretty_type(self.names, &subst, &actual);
                self.messages
                    .at(span)
                    .tyck_incompatible(Some(expected), Some(actual));
            }
        }
    }

    fn occurs(var: &UniVar, ty: &Type) -> bool {
        match ty {
            Type::Invalid | Type::Number => false,
            Type::Range(_, _) => false,
            Type::Name(_) => false,
            Type::Fun(t, u) => Self::occurs(var, t) || Self::occurs(var, u),
            Type::Product(t, u) => Self::occurs(var, t) || Self::occurs(var, u),
            Type::Instantiated(ty, mapping) => {
                mapping.values().any(|ty| Self::occurs(var, ty)) || Self::occurs(var, ty)
            }
            Type::Var(_, war) => var == war,
        }
    }

    fn get(&self, mutability: Mutability, var: &UniVar) -> Option<(&HashMap<Name, Type>, Type)> {
        self.subst
            .get(var)
            .map(|(inst, ty)| (inst, ty.make_mutability(mutability)))
    }

    fn set(&mut self, inst: &HashMap<Name, Type>, var: UniVar, ty: Type) {
        assert!(self.subst.insert(var, (inst.clone(), ty)).is_none());
    }
}
