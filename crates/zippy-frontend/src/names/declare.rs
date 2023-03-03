use std::collections::HashMap;

use zippy_common::messages::MessageMaker;
use zippy_common::names::{DeclarableName, ItemName, RawName, UnnamableName, UnnamableNameKind};
use zippy_common::source::Span;

use crate::ast::{AstSource, Expression, Item, Module, Pattern, PatternNode};
use crate::messages::NameMessages;
use crate::Db;

/// Get every name declared within this module.
#[salsa::tracked]
pub fn declared_names(db: &dyn Db, module: Module) -> HashMap<DeclarableName, Span> {
    let root = module.name(db);
    let mut declarer = Declarer::new(db, root);

    for source in module.sources(db) {
        declarer.declare_source(*source);
    }

    declarer.names
}

struct Declarer<'db> {
    db: &'db dyn Db,
    scope: (Vec<DeclarableName>, DeclarableName),
    names: HashMap<DeclarableName, Span>,
}

impl<'db> Declarer<'db> {
    pub fn new(db: &'db dyn Db, root: ItemName) -> Self {
        let scope = DeclarableName::Item(root);

        Self {
            db,
            scope: (Vec::new(), scope),
            names: HashMap::new(),
        }
    }

    pub fn declare_source(&mut self, source: AstSource) {
        for item in source.items(self.db) {
            self.declare_item(item);
        }
    }

    fn declare_item(&mut self, item: &Item) {
        match item {
            Item::Let {
                pattern,
                anno: _,
                body,
            } => {
                let name = self.declare_pattern(pattern, |declarer, name| {
                    DeclarableName::Item(ItemName::new(
                        declarer.common_db(),
                        Some(declarer.scope.1),
                        name,
                    ))
                });

                let name = match name {
                    Some(name) => name,
                    None => {
                        let name = UnnamableName::new(
                            self.common_db(),
                            UnnamableNameKind::Pattern,
                            Some(self.scope.1),
                            pattern.span,
                        );
                        DeclarableName::Unnamable(name)
                    }
                };

                if let Some(body) = body {
                    self.within(name, |declarer| {
                        declarer.declare_expression(body);
                    });
                }
            }
        }
    }

    fn declare_pattern<F>(&mut self, pattern: &Pattern, mut f: F) -> Option<DeclarableName>
    where
        F: FnMut(&Self, RawName) -> DeclarableName,
    {
        let span = pattern.span;
        match &pattern.node {
            PatternNode::Annotate(pattern, _) => self.declare_pattern(pattern, f),

            PatternNode::Name(name) => {
                let name = f(self, name.name);
                self.try_declare_name(name, span);
                Some(name)
            }

            PatternNode::Unit => None,
            PatternNode::Invalid(_) => None,
        }
    }

    fn declare_expression(&mut self, _expression: &Expression) {
        // empty
    }

    /// Try to declare a name, and produce an error message if it already has
    /// been declared.
    fn try_declare_name(&mut self, name: DeclarableName, span: Span) {
        if let Some(previous) = self.names.get(&name) {
            self.at(span)
                .duplicate_definition(name.to_name(), *previous);

            return;
        }

        self.names.insert(name, span);
    }

    /// Declare some names within the scope of another one.
    fn within<F, T>(&mut self, name: DeclarableName, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.scope.0.push(self.scope.1);
        self.scope.1 = name;

        let result = f(self);

        self.scope.1 = self
            .scope
            .0
            .pop()
            .expect("`self.scope` modified outside `self.within()`");
        result
    }

    fn at(&self, span: Span) -> MessageMaker<&'db dyn Db> {
        MessageMaker::new(self.db, span)
    }

    /// Get a database usable with functions from [`zippy_common`].
    fn common_db(&self) -> &'db dyn zippy_common::Db {
        <dyn Db as salsa::DbWithJar<zippy_common::Jar>>::as_jar_db(self.db)
    }
}
