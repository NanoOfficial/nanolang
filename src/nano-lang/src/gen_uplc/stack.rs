/**
 * @file stack.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-11
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{rc::Rc, sync::Arc};
use indexmap::IndexSet;
use untyped_plutus_core::{builder::EXPECT_ON_LIST, builtins::DefaultFunction};
use crate::{
    ast::Span,
    builtins::{data, list, void},
    tipo::{Type, ValueConstructor, ValueConstructorVariant},
    IdGenerator,
};
use super::{air::Air, scope::Scope};

#[derive(Debug)]
pub struct AirStack {
    pub id_gen: Rc<IdGenerator>,
    pub scope: Scope,
    pub air: Vec<Air>,
}

impl AirStack {
    pub fn new(id_gen: Rc<IdGenerator>) -> Self {
        AirStack {
            id_gen,
            scope: Scope::default(),
            air: vec![],
        }
    }

    pub fn with_scope(id_gen: Rc<IdGenerator>, scope: Scope) -> Self {
        AirStack {
            id_gen,
            scope,
            air: vec![],
        }
    }

    pub fn empty_with_scope(&mut self) -> Self {
        AirStack::with_scope(self.id_gen.clone(), self.scope.clone())
    }

    fn new_scope(&mut self) {
        self.scope.push(self.id_gen.next());
    }

    pub fn merge(&mut self, mut other: AirStack) {
        self.air.append(&mut other.air);
    }

    pub fn merge_child(&mut self, mut other: AirStack) {
        for ir in other.air.iter_mut() {
            ir.scope_mut().replace(self.scope.clone());
        }

        self.merge(other);
    }

    pub fn merge_children(&mut self, stacks: Vec<AirStack>) {
        for stack in stacks {
            self.merge_child(stack)
        }
    }

    pub fn complete(self) -> Vec<Air> {
        self.air
    }

    pub fn sequence(&mut self, stacks: Vec<AirStack>) {
        for stack in stacks {
            self.merge(stack)
        }
    }

    pub fn integer(&mut self, value: String) {
        self.new_scope();

        self.air.push(Air::Int {
            scope: self.scope.clone(),
            value,
        });
    }

    pub fn string(&mut self, value: impl ToString) {
        self.new_scope();

        self.air.push(Air::String {
            scope: self.scope.clone(),
            value: value.to_string(),
        });
    }

    pub fn byte_array(&mut self, bytes: Vec<u8>) {
        self.new_scope();

        self.air.push(Air::ByteArray {
            scope: self.scope.clone(),
            bytes,
        });
    }

    pub fn builtin(&mut self, func: DefaultFunction, tipo: Arc<Type>, args: Vec<AirStack>) {
        self.new_scope();

        self.air.push(Air::Builtin {
            scope: self.scope.clone(),
            count: args.len(),
            func,
            tipo,
        });

        self.merge_children(args);
    }

    pub fn var(
        &mut self,
        constructor: ValueConstructor,
        name: impl ToString,
        variant_name: impl ToString,
    ) {
        self.new_scope();

        self.air.push(Air::Var {
            scope: self.scope.clone(),
            constructor,
            name: name.to_string(),
            variant_name: variant_name.to_string(),
        });
    }

    pub fn local_var(&mut self, tipo: Arc<Type>, name: impl ToString) {
        self.new_scope();

        self.air.push(Air::Var {
            scope: self.scope.clone(),
            constructor: ValueConstructor::public(
                tipo,
                ValueConstructorVariant::LocalVariable {
                    location: Span::empty(),
                },
            ),
            name: name.to_string(),
            variant_name: String::new(),
        });
    }

    pub fn anonymous_function(&mut self, params: Vec<String>, body: AirStack) {
        self.new_scope();

        self.air.push(Air::Fn {
            scope: self.scope.clone(),
            params,
        });

        self.merge_child(body);
    }

    pub fn list(&mut self, tipo: Arc<Type>, elements: Vec<AirStack>, tail: Option<AirStack>) {
        self.new_scope();

        self.air.push(Air::List {
            scope: self.scope.clone(),
            count: elements.len(),
            tipo,
            tail: tail.is_some(),
        });

        self.merge_children(elements);

        if let Some(tail) = tail {
            self.merge_child(tail);
        }
    }

    pub fn record(&mut self, tipo: Arc<Type>, tag: usize, fields: Vec<AirStack>) {
        self.new_scope();

        self.air.push(Air::Record {
            scope: self.scope.clone(),
            tag,
            tipo,
            count: fields.len(),
        });

        self.merge_children(fields);
    }

    pub fn call(&mut self, tipo: Arc<Type>, fun: AirStack, args: Vec<AirStack>) {
        self.new_scope();

        self.air.push(Air::Call {
            scope: self.scope.clone(),
            count: args.len(),
            tipo,
        });

        self.merge_child(fun);

        self.merge_children(args);
    }

    pub fn binop(
        &mut self,
        name: crate::ast::BinOp,
        tipo: Arc<Type>,
        left: AirStack,
        right: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::BinOp {
            scope: self.scope.clone(),
            name,
            tipo,
        });

        self.merge_child(left);
        self.merge_child(right);
    }

    pub fn unop(&mut self, op: crate::ast::UnOp, value: AirStack) {
        self.new_scope();

        self.air.push(Air::UnOp {
            scope: self.scope.clone(),
            op,
        });

        self.merge_child(value);
    }

    pub fn let_assignment(&mut self, name: impl ToString, value: AirStack) {
        self.new_scope();

        self.air.push(Air::Let {
            scope: self.scope.clone(),
            name: name.to_string(),
        });

        self.merge_child(value);
    }

    pub fn expect_list_from_data(
        &mut self,
        tipo: Arc<Type>,
        name: impl ToString,
        unwrap_function: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::Builtin {
            scope: self.scope.clone(),
            func: DefaultFunction::ChooseUnit,
            tipo: tipo.clone(),
            count: DefaultFunction::ChooseUnit.arity(),
        });

        self.new_scope();

        self.air.push(Air::Call {
            scope: self.scope.clone(),
            count: 2,
            tipo: tipo.clone(),
        });

        self.var(
            ValueConstructor::public(
                void(),
                ValueConstructorVariant::ModuleFn {
                    name: EXPECT_ON_LIST.to_string(),
                    field_map: None,
                    module: "".to_string(),
                    arity: 2,
                    location: Span::empty(),
                    builtin: None,
                },
            ),
            EXPECT_ON_LIST,
            "",
        );

        self.local_var(tipo, name);

        self.merge_child(unwrap_function);
    }

    pub fn wrap_data(&mut self, tipo: Arc<Type>) {
        self.new_scope();

        self.air.push(Air::WrapData {
            scope: self.scope.clone(),
            tipo,
        })
    }

    pub fn un_wrap_data(&mut self, tipo: Arc<Type>) {
        self.new_scope();

        self.air.push(Air::UnWrapData {
            scope: self.scope.clone(),
            tipo,
        })
    }

    pub fn void(&mut self) {
        self.new_scope();

        self.air.push(Air::Void {
            scope: self.scope.clone(),
        })
    }

    pub fn tuple_accessor(
        &mut self,
        tipo: Arc<Type>,
        names: Vec<String>,
        check_last_item: bool,
        value: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::TupleAccessor {
            scope: self.scope.clone(),
            names,
            tipo,
            check_last_item,
        });

        self.merge_child(value);
    }

    pub fn fields_expose(
        &mut self,
        indices: Vec<(usize, String, Arc<Type>)>,
        check_last_item: bool,
        value: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::FieldsExpose {
            scope: self.scope.clone(),
            indices,
            check_last_item,
        });

        self.merge_child(value);
    }

    pub fn fields_empty(&mut self, value: AirStack) {
        self.new_scope();

        self.air.push(Air::FieldsEmpty {
            scope: self.scope.clone(),
        });

        self.merge_child(value);
    }

    pub fn clause(
        &mut self,
        tipo: Arc<Type>,
        subject_name: impl ToString,
        complex_clause: bool,
        body: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::Clause {
            scope: self.scope.clone(),
            subject_name: subject_name.to_string(),
            complex_clause,
            tipo,
        });

        self.merge_child(body);
    }

    pub fn list_clause(
        &mut self,
        tipo: Arc<Type>,
        tail_name: impl ToString,
        next_tail_name: Option<String>,
        complex_clause: bool,
        body: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::ListClause {
            scope: self.scope.clone(),
            tail_name: tail_name.to_string(),
            next_tail_name,
            complex_clause,
            tipo,
        });

        self.merge_child(body);
    }

    pub fn tuple_clause(
        &mut self,
        tipo: Arc<Type>,
        subject_name: impl ToString,
        indices: IndexSet<(usize, String)>,
        predefined_indices: IndexSet<(usize, String)>,
        complex_clause: bool,
        body: AirStack,
    ) {
        self.new_scope();

        let count = tipo.get_inner_types().len();

        self.air.push(Air::TupleClause {
            scope: self.scope.clone(),
            subject_name: subject_name.to_string(),
            indices,
            predefined_indices,
            complex_clause,
            tipo,
            count,
        });

        self.merge_child(body);
    }

    pub fn wrap_clause(&mut self, body: AirStack) {
        self.new_scope();

        self.air.push(Air::WrapClause {
            scope: self.scope.clone(),
        });

        self.merge_child(body);
    }

    pub fn trace(&mut self, tipo: Arc<Type>) {
        self.new_scope();

        self.air.push(Air::Trace {
            scope: self.scope.clone(),
            tipo,
        })
    }

    pub fn error(&mut self, tipo: Arc<Type>) {
        self.new_scope();

        self.air.push(Air::ErrorTerm {
            scope: self.scope.clone(),
            tipo,
        })
    }

    pub fn expect_constr_from_data(&mut self, tipo: Arc<Type>, when_stack: AirStack) {
        self.new_scope();

        self.air.push(Air::Builtin {
            scope: self.scope.clone(),
            func: DefaultFunction::ChooseUnit,
            tipo,
            count: DefaultFunction::ChooseUnit.arity(),
        });

        self.merge_child(when_stack);
    }

    pub fn when(
        &mut self,
        tipo: Arc<Type>,
        subject_name: impl ToString,
        subject_stack: AirStack,
        clauses_stack: AirStack,
        else_stack: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::When {
            scope: self.scope.clone(),
            subject_name: subject_name.to_string(),
            tipo,
        });

        self.merge_child(subject_stack);
        self.merge_child(clauses_stack);
        self.merge_child(else_stack);
    }

    pub fn list_accessor(
        &mut self,
        tipo: Arc<Type>,
        names: Vec<String>,
        tail: bool,
        check_last_item: bool,
        value: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::ListAccessor {
            scope: self.scope.clone(),
            names,
            tail,
            check_last_item,
            tipo,
        });

        self.merge_child(value);
    }

    pub fn expect_constr(&mut self, tag: usize, value: AirStack) {
        self.new_scope();

        self.air.push(Air::AssertConstr {
            scope: self.scope.clone(),
            constr_index: tag,
        });

        self.merge_child(value);
    }

    pub fn expect_bool(&mut self, is_true: bool, value: AirStack) {
        self.new_scope();

        self.air.push(Air::AssertBool {
            scope: self.scope.clone(),
            is_true,
        });

        self.merge_child(value);
    }

    pub fn if_branch(&mut self, tipo: Arc<Type>, condition: AirStack, branch_body: AirStack) {
        self.new_scope();

        self.air.push(Air::If {
            scope: self.scope.clone(),
            tipo,
        });

        self.merge_child(condition);
        self.merge_child(branch_body);
    }

    pub fn record_access(&mut self, tipo: Arc<Type>, record_index: u64, record: AirStack) {
        self.new_scope();

        self.air.push(Air::RecordAccess {
            scope: self.scope.clone(),
            record_index,
            tipo,
        });

        self.merge_child(record);
    }

    pub fn record_update(
        &mut self,
        tipo: Arc<Type>,
        highest_index: usize,
        indices: Vec<(usize, Arc<Type>)>,
        update: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::RecordUpdate {
            scope: self.scope.clone(),
            highest_index,
            indices,
            tipo,
        });

        self.merge_child(update);
    }

    pub fn tuple(&mut self, tipo: Arc<Type>, elems: Vec<AirStack>) {
        self.new_scope();

        self.air.push(Air::Tuple {
            scope: self.scope.clone(),
            count: elems.len(),
            tipo,
        });

        self.merge_children(elems);
    }

    pub fn tuple_index(&mut self, tipo: Arc<Type>, tuple_index: usize, tuple: AirStack) {
        self.new_scope();

        self.air.push(Air::TupleIndex {
            scope: self.scope.clone(),
            tuple_index,
            tipo,
        });

        self.merge_child(tuple);
    }

    pub fn finally(&mut self, value: AirStack) {
        self.new_scope();

        self.air.push(Air::Finally {
            scope: self.scope.clone(),
        });

        self.merge_child(value);
    }

    pub fn bool(&mut self, value: bool) {
        self.new_scope();

        self.air.push(Air::Bool {
            scope: self.scope.clone(),
            value,
        });
    }

    pub fn clause_guard(
        &mut self,
        subject_name: impl ToString,
        tipo: Arc<Type>,
        condition_stack: AirStack,
        clause_then_stack: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::ClauseGuard {
            scope: self.scope.clone(),
            subject_name: subject_name.to_string(),
            tipo,
        });

        self.merge_child(condition_stack);

        self.merge_child(clause_then_stack);
    }

    pub fn list_expose(
        &mut self,
        tipo: Arc<Type>,
        tail_head_names: Vec<(String, String)>,
        tail: Option<(String, String)>,
        value: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::ListExpose {
            scope: self.scope.clone(),
            tipo,
            tail_head_names,
            tail,
        });

        self.merge_child(value);
    }

    pub fn list_clause_guard(
        &mut self,
        tipo: Arc<Type>,
        tail_name: impl ToString,
        next_tail_name: Option<String>,
        inverse: bool,
        void_stack: AirStack,
    ) {
        self.new_scope();

        self.air.push(Air::ListClauseGuard {
            scope: self.scope.clone(),
            tipo,
            tail_name: tail_name.to_string(),
            next_tail_name,
            inverse,
        });

        self.merge_child(void_stack);
    }

    pub fn define_func(
        &mut self,
        func_name: impl ToString,
        module_name: impl ToString,
        variant_name: impl ToString,
        params: Vec<String>,
        recursive: bool,
        body_stack: AirStack,
    ) {
        self.air.push(Air::DefineFunc {
            scope: self.scope.clone(),
            func_name: func_name.to_string(),
            module_name: module_name.to_string(),
            params,
            recursive,
            variant_name: variant_name.to_string(),
        });

        self.merge_child(body_stack);
    }

    pub fn noop(&mut self) {
        self.new_scope();

        self.air.push(Air::NoOp {
            scope: self.scope.clone(),
        });
    }

    pub fn choose_unit(&mut self, value_stack: AirStack) {
        self.new_scope();

        self.air.push(Air::Builtin {
            scope: self.scope.clone(),
            func: DefaultFunction::ChooseUnit,
            tipo: void(),
            count: DefaultFunction::ChooseUnit.arity(),
        });

        self.merge_child(value_stack);
    }

    pub fn expect_on_list(&mut self) {
        let mut head_stack = self.empty_with_scope();
        let mut tail_stack = self.empty_with_scope();
        let mut check_with_stack = self.empty_with_scope();
        let mut expect_stack = self.empty_with_scope();
        let mut var_stack = self.empty_with_scope();
        let mut void_stack = self.empty_with_scope();
        let mut fun_stack = self.empty_with_scope();
        let mut arg_stack1 = self.empty_with_scope();
        let mut arg_stack2 = self.empty_with_scope();

        self.air.push(Air::DefineFunc {
            scope: self.scope.clone(),
            func_name: EXPECT_ON_LIST.to_string(),
            module_name: "".to_string(),
            params: vec!["__list_to_check".to_string(), "__check_with".to_string()],
            recursive: true,
            variant_name: "".to_string(),
        });

        var_stack.local_var(list(data()), "__list_to_check");

        head_stack.builtin(DefaultFunction::HeadList, data(), vec![var_stack]);

        fun_stack.local_var(void(), "__check_with".to_string());

        check_with_stack.call(void(), fun_stack, vec![head_stack]);

        void_stack.void();
        void_stack.void();

        self.list_clause(void(), "__list_to_check", None, false, void_stack);

        self.choose_unit(check_with_stack);

        expect_stack.var(
            ValueConstructor::public(
                void(),
                ValueConstructorVariant::ModuleFn {
                    name: EXPECT_ON_LIST.to_string(),
                    field_map: None,
                    module: "".to_string(),
                    arity: 2,
                    location: Span::empty(),
                    builtin: None,
                },
            ),
            EXPECT_ON_LIST,
            "",
        );

        arg_stack1.local_var(list(data()), "__list_to_check");

        arg_stack2.local_var(void(), "__check_with");

        tail_stack.builtin(DefaultFunction::TailList, list(data()), vec![arg_stack1]);

        self.call(void(), expect_stack, vec![tail_stack, arg_stack2])
    }

    pub fn list_empty(&mut self, value_stack: AirStack) {
        self.new_scope();

        self.air.push(Air::ListEmpty {
            scope: self.scope.clone(),
        });

        self.merge_child(value_stack);
    }
}