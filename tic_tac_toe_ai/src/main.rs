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

    let mut model = Model::new(rand::thread_rng(), 0.6, 0.9);
    let periods = 1000000;
    for i in 0..periods {
        let verbose = i % (periods / 10000) == 0;
        model.run_episode(verbose);
    }
    model.dump_table(&output_path).unwrap();
}
