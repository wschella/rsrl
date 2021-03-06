use crate::spaces::{ProductSpace, real::Interval, discrete::Ordinal};
use super::{Domain, Observation, Transition, runge_kutta4};

// Model parameters
// (https://pdfs.semanticscholar.org/c030/127238b1dbad2263fba6b64b5dec7c3ffa20.pdf):
const LAMBDA1: f64 = 1e4;
const LAMBDA2: f64 = 31.98;
const D1: f64 = 0.01;
const D2: f64 = 0.01;
const F: f64 = 0.34;
const K1: f64 = 8e-7;
const K2: f64 = 1e-4;
const DELTA: f64 = 0.7;
const M1: f64 = 1e-5;
const M2: f64 = 1e-5;
const NT: f64 = 100.0;
const C: f64 = 13.0;
const RHO1: f64 = 1.0;
const RHO2: f64 = 1.0;
const LAMBDA_E: f64 = 1.0;
const BE: f64 = 0.3;
const KB: f64 = 100.0;
const DE: f64 = 0.25;
const KD: f64 = 500.0;
const DELTA_E: f64 = 0.1;

// Simulation parameters:
const DT: f64 = 5.0;
const SIM_STEPS: u32 = 1000;

const DT_STEP: f64 = DT / SIM_STEPS as f64;

// RL parameters:
const LIMITS: [f64; 2] = [-5.0, 8.0];
const ALL_ACTIONS: [[f64; 2]; 4] = [[0.0, 0.0], [0.7, 0.0], [0.0, 0.3], [0.7, 0.3]];

make_index!(StateIndex [
    T1 => 0, T1S => 1, T2 => 2, T2S => 3, V => 4, E => 5
]);

pub struct HIVTreatment {
    eps: [f64; 2],
    state: [f64; 6],
}

impl HIVTreatment {
    pub fn new(t1: f64, t1s: f64, t2: f64, t2s: f64, v: f64, e: f64) -> HIVTreatment {
        HIVTreatment {
            eps: ALL_ACTIONS[0],
            state: [t1, t1s, t2, t2s, v, e],
        }
    }

    fn update_state(&mut self, a: usize) {
        let eps = ALL_ACTIONS[a];
        let fx = |_x, y| HIVTreatment::grad(eps, y);

        self.eps = eps;

        let mut ns = runge_kutta4(&fx, 0.0, self.state.to_vec(), DT_STEP);
        for _ in 1..SIM_STEPS {
            ns = runge_kutta4(&fx, 0.0, ns, DT_STEP);
        }

        self.state[StateIndex::T1] = ns[StateIndex::T1];
        self.state[StateIndex::T1S] = ns[StateIndex::T1S];
        self.state[StateIndex::T2] = ns[StateIndex::T2];
        self.state[StateIndex::T2S] = ns[StateIndex::T2S];
        self.state[StateIndex::V] = ns[StateIndex::V];
        self.state[StateIndex::E] = ns[StateIndex::E];
    }

    fn grad(eps: [f64; 2], mut buffer: Vec<f64>) -> Vec<f64> {
        let t1 = buffer[StateIndex::T1];
        let t1s = buffer[StateIndex::T1S];
        let t2 = buffer[StateIndex::T2];
        let t2s = buffer[StateIndex::T2S];
        let v = buffer[StateIndex::V];
        let e = buffer[StateIndex::E];

        let tmp1 = (1.0 - eps[0]) * K1 * v * t1;
        let tmp2 = (1.0 - F * eps[0]) * K2 * v * t2;
        let sum_ts = t1s + t2s;

        buffer[StateIndex::T1] = LAMBDA1 - D1 * t1 - tmp1;
        buffer[StateIndex::T1S] = tmp1 - DELTA * t1s - M1 * e * t1s;

        buffer[StateIndex::T2] = LAMBDA2 - D2 * t2 - tmp2;
        buffer[StateIndex::T2S] = tmp2 - DELTA * t2s - M2 * e * t2s;

        buffer[StateIndex::V] = (1.0 - eps[1]) * NT * DELTA * sum_ts - C * v
            - ((1.0 - eps[0]) * RHO1 * K1 * t1 + (1.0 - F * eps[0]) * RHO2 * K2 * t2) * v;
        buffer[StateIndex::E] =
            LAMBDA_E + BE * sum_ts / (sum_ts + KB) * e
            - DE * sum_ts / (sum_ts + KD) * e - DELTA_E * e;

        buffer
    }
}

impl Default for HIVTreatment {
    fn default() -> HIVTreatment { HIVTreatment::new(163_573.0, 11_945.0, 5.0, 46.0, 63_919.0, 24.0) }
}

impl Domain for HIVTreatment {
    type StateSpace = ProductSpace<Interval>;
    type ActionSpace = Ordinal;

    fn emit(&self) -> Observation<Vec<f64>> {
        let s = self.state.iter().map(|v| clip!(LIMITS[0], v.log10(), LIMITS[1]));

        Observation::Full(s.collect())
    }

    fn step(&mut self, action: usize) -> Transition<Vec<f64>, usize> {
        let from = self.emit();

        self.update_state(action);

        let to = self.emit();

        Transition {
            from,
            action,
            reward: to.map_into(|s| {
                let r = 1e3 * s[StateIndex::E] - 0.1 * s[StateIndex::V] - 2e4 * self.eps[0].powi(2)
                    - 2e3 * self.eps[1].powi(2);

                r / 1e5
            }),
            to,
        }
    }

    fn state_space(&self) -> Self::StateSpace {
        ProductSpace::empty() +
            Interval::bounded(LIMITS[0], LIMITS[1])
            + Interval::bounded(LIMITS[0], LIMITS[1])
            + Interval::bounded(LIMITS[0], LIMITS[1])
            + Interval::bounded(LIMITS[0], LIMITS[1])
            + Interval::bounded(LIMITS[0], LIMITS[1])
            + Interval::bounded(LIMITS[0], LIMITS[1])
    }

    fn action_space(&self) -> Ordinal { Ordinal::new(4) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_observation() {
        let m = HIVTreatment::new(1.0, 10.0, 100.0, 200.0, 500.0, 10000.0);

        match m.emit() {
            Observation::Full(ref state) => {
                assert!((state[0] - 0.0).abs() < 1e-7);
                assert!((state[1] - 1.0).abs() < 1e-7);
                assert!((state[2] - 2.0).abs() < 1e-7);
                assert!((state[3] - 2.301029995663981).abs() < 1e-7);
                assert!((state[4] - 2.698970004336019).abs() < 1e-7);
                assert!((state[5] - 4.0).abs() < 1e-7);
            },
            _ => panic!("Should yield a fully observable state."),
        }
    }

    #[test]
    fn test_initial_observation_default() {
        let m = HIVTreatment::default();

        match m.emit() {
            Observation::Full(ref state) => {
                assert!((state[0] - 5.213711618903007).abs() < 1e-7);
                assert!((state[1] - 4.077186154085897).abs() < 1e-7);
                assert!((state[2] - 0.698970004336019).abs() < 1e-7);
                assert!((state[3] - 1.662757831681574).abs() < 1e-7);
                assert!((state[4] - 4.805629971908577).abs() < 1e-7);
                assert!((state[5] - 1.380211241711606).abs() < 1e-7);
            },
            _ => panic!("Should yield a fully observable state."),
        }
    }

    #[test]
    fn test_limits() {
        let m = HIVTreatment::new(1e10, 1e-10, 1.0, 1.0, 1.0, 1.0);

        match m.emit() {
            Observation::Full(ref state) => {
                assert!((state[0] - LIMITS[1]).abs() < 1e-7);
                assert!((state[1] - LIMITS[0]).abs() < 1e-7);
                assert!((state[2] - 0.0).abs() < 1e-7);
                assert!((state[3] - 0.0).abs() < 1e-7);
                assert!((state[4] - 0.0).abs() < 1e-7);
                assert!((state[5] - 0.0).abs() < 1e-7);
            },
            _ => panic!("Should yield a fully observable state."),
        }
    }
}
