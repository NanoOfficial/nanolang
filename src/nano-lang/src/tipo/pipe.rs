/**
 * @file pipe.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{ops::Deref, sync::Arc};
use vec1::Vec1;
use crate::{
    ast::{AssignmentKind, CallArg, Pattern, Span, PIPE_VARIABLE},
    builtins::function,
    expr::{TypedExpr, UntypedExpr},
};
use super::{
    error::{Error, UnifyErrorSituation},
    expr::ExprTyper,
    Type, ValueConstructor, ValueConstructorVariant,
};

#[derive(Debug)]
pub(crate) struct PipeTyper<'a, 'b, 'c> {
    size: usize,
    argument_type: Arc<Type>,
    argument_location: Span,
    location: Span,
    expressions: Vec<TypedExpr>,
    expr_typer: &'a mut ExprTyper<'b, 'c>,
}

impl<'a, 'b, 'c> PipeTyper<'a, 'b, 'c> {
    pub fn infer(
        expr_typer: &'a mut ExprTyper<'b, 'c>,
        expressions: Vec1<UntypedExpr>,
    ) -> Result<TypedExpr, Error> {
        let size = expressions.len();

        let end = &expressions[..]
            .last()
            .expect("Empty pipeline in typer")
            .location()
            .end;

        let mut expressions = expressions.into_iter();

        let first = expr_typer.infer(expressions.next().expect("Empty pipeline in typer"))?;

        let mut typer = Self {
            size,
            expr_typer,
            argument_type: first.tipo(),
            argument_location: first.location(),
            location: Span {
                start: first.location().start,
                end: *end,
            },
            expressions: Vec::with_capacity(size),
        };

        typer.push_assignment_no_update(first);

        typer.infer_expressions(expressions)
    }

    fn infer_expressions(
        mut self,
        expressions: impl IntoIterator<Item = UntypedExpr>,
    ) -> Result<TypedExpr, Error> {
        let result = self.infer_each_expression(expressions);

        let _ = self.expr_typer.environment.scope.remove(PIPE_VARIABLE);

        result?;

        Ok(TypedExpr::Pipeline {
            expressions: self.expressions,
            location: self.location,
        })
    }

    fn infer_each_expression(
        &mut self,
        expressions: impl IntoIterator<Item = UntypedExpr>,
    ) -> Result<(), Error> {
        for (i, call) in expressions.into_iter().enumerate() {
            let call = match call {
                UntypedExpr::Call {
                    fun,
                    arguments,
                    location,
                    ..
                } => {
                    let fun = self.expr_typer.infer(*fun)?;

                    match fun.tipo().fn_arity() {
                        Some(arity) if arity == arguments.len() + 1 => {
                            self.infer_insert_pipe(fun, arguments, location)?
                        }

                        _ => self.infer_apply_to_call_pipe(fun, arguments, location)?,
                    }
                }

                call => self.infer_apply_pipe(call)?,
            };

            if i + 2 == self.size {
                self.expressions.push(call);
            } else {
                self.push_assignment(call);
            }
        }

        Ok(())
    }

    fn typed_left_hand_value_variable_call_argument(&self) -> CallArg<TypedExpr> {
        CallArg {
            label: None,
            location: self.argument_location,
            value: self.typed_left_hand_value_variable(),
        }
    }

    fn untyped_left_hand_value_variable_call_argument(&self) -> CallArg<UntypedExpr> {
        CallArg {
            label: None,
            location: self.argument_location,
            value: self.untyped_left_hand_value_variable(),
        }
    }

    fn typed_left_hand_value_variable(&self) -> TypedExpr {
        TypedExpr::Var {
            location: self.argument_location,
            name: PIPE_VARIABLE.to_string(),
            constructor: ValueConstructor {
                public: true,
                tipo: self.argument_type.clone(),
                variant: ValueConstructorVariant::LocalVariable {
                    location: self.argument_location,
                },
            },
        }
    }

    fn untyped_left_hand_value_variable(&self) -> UntypedExpr {
        UntypedExpr::Var {
            location: self.argument_location,
            name: PIPE_VARIABLE.to_string(),
        }
    }

    fn push_assignment(&mut self, expression: TypedExpr) {
        self.argument_type = expression.tipo();
        self.argument_location = expression.location();
        self.push_assignment_no_update(expression)
    }

    fn push_assignment_no_update(&mut self, expression: TypedExpr) {
        let location = expression.location();

        self.expr_typer.environment.insert_variable(
            PIPE_VARIABLE.to_string(),
            ValueConstructorVariant::LocalVariable { location },
            expression.tipo(),
        );

        let assignment = TypedExpr::Assignment {
            location,
            tipo: expression.tipo(),
            kind: AssignmentKind::Let,
            value: Box::new(expression),
            pattern: Pattern::Var {
                location,
                name: PIPE_VARIABLE.to_string(),
            },
        };

        self.expressions.push(assignment);
    }

    fn infer_apply_to_call_pipe(
        &mut self,
        function: TypedExpr,
        args: Vec<CallArg<UntypedExpr>>,
        location: Span,
    ) -> Result<TypedExpr, Error> {
        let (function, args, tipo) = self
            .expr_typer
            .do_infer_call_with_known_fun(function, args, location)?;

        let function = TypedExpr::Call {
            location,
            tipo,
            args,
            fun: Box::new(function),
        };

        let args = vec![self.untyped_left_hand_value_variable_call_argument()];

        let (function, args, tipo) = self
            .expr_typer
            .do_infer_call_with_known_fun(function, args, location)?;

        Ok(TypedExpr::Call {
            location,
            tipo,
            args,
            fun: Box::new(function),
        })
    }

    fn infer_insert_pipe(
        &mut self,
        function: TypedExpr,
        mut arguments: Vec<CallArg<UntypedExpr>>,
        location: Span,
    ) -> Result<TypedExpr, Error> {
        arguments.insert(0, self.untyped_left_hand_value_variable_call_argument());
    
        let (fun, args, tipo) = self
            .expr_typer
            .do_infer_call_with_known_fun(function, arguments, location)?;

        Ok(TypedExpr::Call {
            location,
            tipo,
            args,
            fun: Box::new(fun),
        })
    }

    fn infer_apply_pipe(&mut self, func: UntypedExpr) -> Result<TypedExpr, Error> {
        let func = Box::new(self.expr_typer.infer(func)?);
        let return_type = self.expr_typer.new_unbound_var();

        self.expr_typer
            .environment
            .unify(
                func.tipo(),
                function(vec![self.argument_type.clone()], return_type.clone()),
                func.location(),
                if let Type::Fn { args, .. } = func.tipo().deref() {
                    if let Some(typ) = args.get(0) {
                        typ.is_data()
                    } else {
                        false
                    }
                } else {
                    false
                },
            )
            .map_err(|e| {
                let is_pipe_mismatch = self.check_if_pipe_type_mismatch(&e, func.location());

                if is_pipe_mismatch {
                    e.with_unify_error_situation(UnifyErrorSituation::PipeTypeMismatch)
                } else {
                    e
                }
            })?;

        Ok(TypedExpr::Call {
            location: func.location(),
            tipo: return_type,
            fun: func,
            args: vec![self.typed_left_hand_value_variable_call_argument()],
        })
    }

    fn check_if_pipe_type_mismatch(&mut self, error: &Error, location: Span) -> bool {
        let types = match error {
            Error::CouldNotUnify {
                expected, given, ..
            } => (expected.as_ref(), given.as_ref()),
            _ => return false,
        };

        match types {
            (Type::Fn { args: a, .. }, Type::Fn { args: b, .. }) if a.len() == b.len() => {
                match (a.get(0), b.get(0)) {
                    (Some(a), Some(b)) => self
                        .expr_typer
                        .environment
                        .unify(a.clone(), b.clone(), location, a.is_data())
                        .is_err(),
                    _ => false,
                }
            }
            _ => false,
        }
    }
}