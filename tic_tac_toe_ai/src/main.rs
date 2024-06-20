mod q_learning;

fn main() {
    let mut model = q_learning::Model::new(0.8, 0.9);
    let periods = 100000;
    for i in 0..periods {
        let verbose = i % (periods / 100) == 0;
        if verbose {
            println!("starting episode {}", i);
        }
        model.run_episode(verbose);
    }
}
