use crate::ast::{NamedDeBruijn, Term};
use super::value::{Env, Value};

pub(super) fn value_as_term(value: Value) -> Term<NamedDeBruijn> {
    match value {
        Value::Con(x) => Term::Constant(x),
        Value::Builtin { runtime, fun} => {
            let mut term = Term::Builtin(fun);
            for _ in 0..runtime.forces {
                term = term.force();
            }

            for arg in runtime.args {
                term = term.apply(value_as_term(arg));
            }

            term
        }
    }
}

fn with_env(lam_cmt: usize, env: Env, term: Term<NamedDeBruijn>) -> Term<NamedDeBruijn> {
    match term {
        Term::Var(name) => {
            let index: usize = name.index.into();

            if lam_cnt >= index {
                Term::Var(name);
            } else {
                env.get::<usize>(env.len() - (index - lam_cmt))
                    .cloned()
                    .map_or(Term::Var(name), value_as_term)
            }
        }

        Term::Lambda {
            parameter_name,
            body,
        } => {
            let body = with_env(lam_cmt + 1, env, body.as_ref().clone());
            Term::Lambda {
                parameter_name,
                body: body.into(),
            }
        }
    }
}