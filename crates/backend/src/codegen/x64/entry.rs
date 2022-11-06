use common::names::{Actual, Name, Path};
use iced_x86::code_asm::{rax, rcx, rdi};
use target_lexicon::{Architecture, BinaryFormat, Environment, OperatingSystem, Triple};

use super::{Error, Lowerer};

impl Lowerer<'_> {
    pub fn entry(&mut self, entry: Name, target: &Triple) -> Result<(), Error> {
        assert!(target.architecture == Architecture::X86_64);

        match target {
            Triple {
                operating_system: OperatingSystem::Windows,
                binary_format: BinaryFormat::Coff | BinaryFormat::Unknown,
                environment: Environment::Msvc | Environment::Unknown,
                ..
            } => self.entry_windows(entry),

            Triple {
                operating_system: OperatingSystem::Linux,
                binary_format: BinaryFormat::Elf | BinaryFormat::Unknown,
                ..
            } => self.entry_linux(entry),

            target => return Err(Error::UnsupportedTarget(target.clone())),
        }

        Ok(())
    }

    fn entry_linux(&mut self, entry: Name) {
        let span = self.names.get_span(&entry);
        let start = self
            .names
            .add(span, Path(None, Actual::Lit("_start".into())));

        let entry = self.label(entry);

        self.set_label(start);
        self.asm.call(entry).unwrap();
        self.asm.mov(rax, 60i64).unwrap();
        self.asm.syscall().unwrap();
    }

    fn entry_windows(&mut self, entry: Name) {
        let span = self.names.get_span(&entry);
        let main = self
            .names
            .add(span, Path(None, Actual::Lit("wmain".into())));
        let exit_process = self
            .names
            .add(span, Path(None, Actual::Lit("ExitProcess".into())));

        let exit_process = self.label(exit_process);
        let entry = self.label(entry);

        self.set_label(main);
        self.asm.call(entry).unwrap();
        self.asm.mov(rcx, rdi).unwrap();
        self.asm.call(exit_process).unwrap();
    }
}
