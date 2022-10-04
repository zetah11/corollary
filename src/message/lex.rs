use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::MessageAdder;

const INVALID_CHARACTER: &str = "EL00";

impl<'a> MessageAdder<'a> {
    pub fn lex_invalid(&mut self) {
        let labels = vec![Label::primary(self.at.file, self.at)];

        self.add(
            Diagnostic::error()
                .with_code(INVALID_CHARACTER)
                .with_message("invalid character")
                .with_labels(labels),
        );
    }
}
