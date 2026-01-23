use std::collections::HashMap;
use std::rc::Rc;

use clvmr::Allocator;

use crate::classic::clvm_tools::stages::stage_0::DefaultProgramRunner;
use crate::compiler::clvm::run;
use crate::compiler::codegen::codegen;
use crate::compiler::compiler::DefaultCompilerOpts;
use crate::compiler::comptypes::CompilerOpts;
use crate::compiler::frontend::frontend;
use crate::compiler::optimize::get_optimizer;
use crate::compiler::resolve::resolve_namespaces;
use crate::compiler::sexp::parse_sexp;
use crate::compiler::srcloc::Srcloc;
use crate::compiler::BasicCompileContext;

#[test]
fn test_compile_module_with_resolver() {
    let test_program =
        "(mod (A) (include *standard-cl-24*) (namespace Z (defconst Q 1)) (namespace X (import qualified Z as Z1) (defun F (Z) (+ Z Z1.Q)) (defun G (Z) (- Z Z1.Q))) (import X hiding G) (namespace Y (defun G (Z) (* Z 2))) (import Y exposing (G as GG)) (F (GG A)))";
    let opts: Rc<dyn CompilerOpts> = Rc::new(DefaultCompilerOpts::new("*resolve-test*"));
    let loc = Srcloc::start("*resolve-test*");
    let parsed = parse_sexp(loc.clone(), test_program.bytes()).unwrap();
    let processed = frontend(opts.clone(), &parsed).unwrap();
    let resolved = resolve_namespaces(opts.clone(), &processed).unwrap();
    let mut context = BasicCompileContext {
        allocator: Allocator::new(),
        runner: Rc::new(DefaultProgramRunner::new()),
        symbols: HashMap::new(),
        optimizer: get_optimizer(&loc, opts.clone()).unwrap(),
    };
    let generated = codegen(&mut context, opts.clone(), &resolved).unwrap();
    let outcome = run(
        &mut context.allocator,
        context.runner.clone(),
        opts.prim_map(),
        Rc::new(generated),
        parse_sexp(loc.clone(), "(3)".bytes()).unwrap()[0].clone(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(outcome.to_string(), "7");
}
