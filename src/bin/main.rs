use haybale_pitchfork::*;
use std::path::Path;

fn main() {
    env_logger::init();
    // Expects 1 argument, the haybale test module name
    let filepath = Path::new("../haybale")
        .join(Path::new("tests"))
        .join(Path::new("bcfiles"))
        .join(Path::new(&std::env::args().nth(1).expect("Please pass an argument")))
        .with_extension("bc");
    let proj = Project::from_bc_path(&filepath).unwrap_or_else(|e| panic!("Failed to parse module at path {}: {}", filepath.display(), e));
    for funcname in proj.all_functions().map(|(f,_)| &f.name) {
        println!("{}", check_for_ct_violation_in_inputs(funcname, &proj, Config::default(), true));
    }
}
