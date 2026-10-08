//! The generated test projects.

pub mod constants;
pub mod events;
pub mod instances;
pub mod lang;
pub mod motion;
pub mod optional;
pub mod perf;
pub mod random;
pub mod restart;
pub mod rooms;
pub mod smoke;

use crate::framework::Spec;

/// Every project, in the order the user should run them.
pub fn all() -> Vec<Spec> {
    let mut v = vec![
        smoke::build1(),
        smoke::build2(),
        smoke::build3(),
        smoke::build4(),
        constants::build(),
        lang::build(),
        lang::build_args0(),
        random::build(),
        events::build(),
        instances::build(),
        motion::build(),
        rooms::build(),
        restart::build(),
        optional::build(),
        optional::build_clash(),
        perf::build(),
        perf::build_fps(),
    ];
    v.extend(lang::diagnostics());
    v
}
