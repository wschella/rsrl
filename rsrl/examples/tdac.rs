extern crate rsrl;
#[macro_use]
extern crate slog;

use rsrl::{
    control::ac::TDAC,
    domains::{ContinuousMountainCar, Domain},
    fa::linear::{
        basis::{Fourier, Projector},
        optim::SGD,
        LFA,
    },
    logging,
    policies::gaussian::{self, Gaussian},
    prediction::lstd::iLSTD,
    run,
    Evaluation,
    SerialExperiment,
};

fn main() {
    let domain = ContinuousMountainCar::default();
    let bases = Fourier::from_space(3, domain.state_space()).with_constant();

    let critic = iLSTD::new(LFA::scalar(bases.clone(), SGD(1.0)), 0.0001, 0.99, 2);
    let policy = Gaussian::new(
        gaussian::mean::Scalar(LFA::scalar(bases, SGD(1.0))),
        gaussian::stddev::Constant(1.0),
    );

    let mut agent = TDAC::new(critic, policy, 0.002, 0.99);

    let logger = logging::root(logging::stdout());
    let domain_builder = Box::new(ContinuousMountainCar::default);

    // Training phase:
    let _training_result = {
        // Start a serial learning experiment up to 1000 steps per episode.
        let e = SerialExperiment::new(&mut agent, domain_builder.clone(), 1000);

        // Realise 2000 episodes of the experiment generator.
        run(e, 2000, Some(logger.clone()))
    };

    // Testing phase:
    let testing_result = Evaluation::new(&mut agent, domain_builder).next().unwrap();

    info!(logger, "solution"; testing_result);
}
