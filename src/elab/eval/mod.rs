mod check;
mod irreducible;
mod promote;
mod reduce;

use std::collections::HashSet;

use im::HashMap;

use crate::message::Messages;
use crate::mir::pretty::Prettier;
use crate::mir::{Context, Decls, Types, ValueDef};
use crate::resolve::names::{Name, Names};
use crate::Driver;

use self::irreducible::{Irreducible, IrreducibleNode};

pub fn evaluate(
    driver: &mut impl Driver,
    context: &mut Context,
    names: &mut Names,
    types: &Types,
    decls: Decls,
    entry: Option<Name>,
) -> Decls {
    let (res, messages) = if let Some(entry) = entry {
        let mut lowerer = Lowerer::new(driver, context, names, types);
        lowerer.discover(decls, entry);
        let res = lowerer.reduce_from();

        //let res = lowerer.reduce_decls(decls);

        (res, lowerer.messages)
    } else {
        (Decls { values: Vec::new() }, Messages::new())
    };

    driver.report(messages);

    res
}

#[derive(Clone, Debug, Default)]
struct Env {
    map: HashMap<Name, Irreducible>,
    parent: Option<Box<Env>>,
}

impl Env {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            parent: None,
        }
    }

    fn child(&self) -> Self {
        Self {
            map: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    fn lookup(&self, name: &Name) -> Option<&Irreducible> {
        self.map
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup(name)))
    }

    fn set(&mut self, name: Name, value: Irreducible) {
        self.map.insert(name, value);
    }

    /// Create a child environment where the given name is bound to the given value.
    fn with(&self, name: Name, value: Irreducible) -> Self {
        Self {
            map: self.map.update(name, value),
            parent: Some(Box::new(self.clone())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Behaviour {
    Discover,
    FullEval,
}

#[derive(Debug)]
struct Lowerer<'a, Driver> {
    env: Env,
    names: &'a mut Names,
    types: &'a Types,
    context: &'a mut Context,

    behaviour: Behaviour,

    driver: &'a mut Driver,
    messages: Messages,

    worklist: Vec<Name>,
}

impl<'a, D: Driver> Lowerer<'a, D> {
    fn new(
        driver: &'a mut D,
        context: &'a mut Context,
        names: &'a mut Names,
        types: &'a Types,
    ) -> Self {
        Self {
            env: Env::new(),
            names,
            types,
            context,

            behaviour: Behaviour::FullEval,

            driver,
            messages: Messages::new(),

            worklist: Vec::new(),
        }
    }

    fn discover(&mut self, decls: Decls, entry: Name) {
        let old_behaviour = self.behaviour;
        self.behaviour = Behaviour::Discover;

        let mut value_defs: std::collections::HashMap<_, _> = decls
            .values
            .into_iter()
            .map(|def| (def.name, def))
            .collect();

        self.worklist.push(entry);
        let mut index = 0;

        while index < self.worklist.len() {
            let name = self.worklist[index];
            index += 1;

            if let Some(def) = value_defs.remove(&name) {
                let bind = self.reduce_exprs(self.env.clone(), def.bind);
                self.env.set(def.name, bind);
            }
        }

        self.behaviour = old_behaviour;
    }

    fn reduce_from(&mut self) -> Decls {
        let mut values = Vec::new();
        let mut value_names = HashSet::new();

        while let Some(name) = self.worklist.pop() {
            if !value_names.contains(&name) && self.env.lookup(&name).is_some() {
                self.driver.report_eval({
                    let prettier = Prettier::new(self.names, self.types);
                    prettier.pretty_name(&name)
                });

                let bind = self.env.lookup(&name).unwrap().clone();
                let bind = self.reduce_irr(self.env.clone(), bind);

                // overwrite with new and improved
                self.env.set(name, bind.clone());

                values.push(ValueDef {
                    name,
                    span: bind.span,
                    bind: self.promote(bind),
                });

                value_names.insert(name);
            }
        }

        self.driver.done_eval();

        Decls { values }
    }

    fn lookup<'b, 'c, 'd>(&'b mut self, env: &'c Env, name: &'d Name) -> Option<&'c Irreducible> {
        match self.behaviour {
            Behaviour::FullEval => env.lookup(name),
            Behaviour::Discover => {
                if !self.worklist.contains(name) {
                    self.worklist.push(*name);
                }
                None
            }
        }
    }
}
