use llvm_ir::Module;
use pitchfork::*;
use std::path::Path;

fn main() {
    env_logger::init();
    // Expects 1 argument, the haybale test module name
    let filepath = Path::new("../haybale")
        .join(Path::new("tests"))
        .join(Path::new("bcfiles"))
        .join(Path::new(&std::env::args().nth(1).expect("Please pass an argument")))
        .with_extension("bc");
    let llvm_mod = Module::from_bc_path(&filepath).unwrap_or_else(|e| panic!("Failed to parse module at path {}: {}", filepath.display(), e));
    for func in &llvm_mod.functions {
        let ct = is_constant_time_in_inputs(func, &llvm_mod, &Config::default());
        println!("{:?} is{} constant-time in its inputs", func.name, if ct {""} else {" not"});
    }
}
