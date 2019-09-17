use crate::{
    Algorithm, OnlineLearner, Parameter,
    control::Controller,
    domains::Transition,
    policies::{Policy, DifferentiablePolicy},
    prediction::{ValuePredictor, ActionValuePredictor},
};
use rand::Rng;

/// Advantage actor-critic.
pub struct A2C<C, P> {
    pub critic: C,
    pub policy: P,

    pub alpha: Parameter,
}

impl<C, P> A2C<C, P> {
    pub fn new<T: Into<Parameter>>(critic: C, policy: P, alpha: T) -> Self {
        A2C {
            critic,
            policy,

            alpha: alpha.into(),
        }
    }
}

impl<C, P> Algorithm for A2C<C, P>
where
    C: Algorithm,
    P: Algorithm,
{
    fn handle_terminal(&mut self) {
        self.alpha = self.alpha.step();

        self.critic.handle_terminal();
        self.policy.handle_terminal();
    }
}

impl<S, C, P> OnlineLearner<S, P::Action> for A2C<C, P>
where
    C: OnlineLearner<S, P::Action> + ValuePredictor<S> + ActionValuePredictor<S, P::Action>,
    P: DifferentiablePolicy<S>,
    P::Action: Clone,
{
    fn handle_transition(&mut self, t: &Transition<S, P::Action>) {
        self.critic.handle_transition(t);

        let s = t.from.state();
        let v = self.critic.predict_v(s);
        let qsa = self.critic.predict_qsa(s, t.action.clone());

        self.policy.update(s, &t.action, self.alpha * (qsa - v));
    }

    fn handle_sequence(&mut self, seq: &[Transition<S, P::Action>]) {
        self.critic.handle_sequence(seq);

        for t in seq {
            let s = t.from.state();
            let v = self.critic.predict_v(s);
            let qsa = self.critic.predict_qsa(s, t.action.clone());

            self.policy.update(s, &t.action, self.alpha * (qsa - v));
        }
    }
}

impl<S, C, P> ValuePredictor<S> for A2C<C, P>
where
    C: ValuePredictor<S>,
{
    fn predict_v(&self, s: &S) -> f64 {
        self.critic.predict_v(s)
    }
}

impl<S, C, P> ActionValuePredictor<S, P::Action> for A2C<C, P>
where
    C: ActionValuePredictor<S, P::Action>,
    P: Policy<S>,
{
    fn predict_qsa(&self, s: &S, a: P::Action) -> f64 {
        self.critic.predict_qsa(s, a)
    }
}

impl<S, C, P> Controller<S, P::Action> for A2C<C, P>
where
    P: Policy<S>,
{
    fn sample_target(&self, rng: &mut impl Rng, s: &S) -> P::Action {
        self.policy.sample(rng, s)
    }

    fn sample_behaviour(&self, rng: &mut impl Rng, s: &S) -> P::Action {
        self.policy.sample(rng, s)
    }
}