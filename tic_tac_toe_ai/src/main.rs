extern crate tic_tac_toe_ai;

use std::env;

use tic_tac_toe_ai::Model;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("output weights file name isn't specified");
        return;
    }
    let output_path = args[1].clone();

    let mut model = Model::new(0.8, 0.9);
    let periods = 100000;
    for i in 0..periods {
        let verbose = i % (periods / 100) == 0;
        if verbose {
            println!("starting episode {}", i);
        }
        model.run_episode(verbose);
    }
    model.dump_table(&output_path).unwrap();
}
