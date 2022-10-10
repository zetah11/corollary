mod console_driver;

use codespan_reporting::files::SimpleFiles;
use corollary::elab::elaborate;
use corollary::lex::lex;
use corollary::mir::pretty::Prettier;
use corollary::parse::parse;
use corollary::resolve::resolve;
use corollary::tyck::typeck;

use console_driver::ConsoleDriver;

fn main() {
    env_logger::init();

    let src = r#"
        let apply: (0 upto 10 -> 0 upto 10) * (0 upto 10) -> 0 upto 10 =
            (f, x) => f x
    "#;
    let mut files = SimpleFiles::new();
    let file = files.add("main.z".into(), src.into());

    let mut driver = ConsoleDriver::new(files);

    let toks = lex(&mut driver, src, file);
    let decls = parse(&mut driver, toks, file);
    let (decls, mut names) = resolve(&mut driver, decls);
    let tyckres = typeck(&mut driver, decls);
    let (_types, decls) = elaborate(&mut driver, &mut names, tyckres);

    let prettier = Prettier::new(&names);

    for def in decls.values.iter() {
        let pat = prettier.pretty_name(&def.name);
        let exp = prettier.pretty_exprs(&def.bind);
        println!("let {pat} = {exp}");
    }
}
