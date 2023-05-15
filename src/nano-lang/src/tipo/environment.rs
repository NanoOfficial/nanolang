/**
 * @file env.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

use itertools::Itertools;

use crate::{
    ast::{
        Annotation, CallArg, DataType, Definition, Function, ModuleConstant, ModuleKind, Pattern,
        RecordConstructor, RecordConstructorArg, Span, TypeAlias, TypedDefinition,
        UnqualifiedImport, UntypedArg, UntypedDefinition, Use, Validator, PIPE_VARIABLE,
    },
    builtins::{self, function, generic_var, tuple, unbound_var},
    tipo::fields::FieldMap,
    IdGenerator,
};

use super::{
    error::{Error, Snippet, Warning},
    hydrator::Hydrator,
    AccessorsMap, PatternConstructor, RecordAccessor, Type, TypeConstructor, TypeInfo, TypeVar,
    ValueConstructor, ValueConstructorVariant,
};

#[derive(Debug)]
pub struct ScopeResetData {
    local_values: HashMap<String, ValueConstructor>,
}

#[derive(Debug)]
pub struct Environment<'a> {
    pub accessors: HashMap<String, AccessorsMap>,
    pub current_module: &'a String,

    pub entity_usages: Vec<HashMap<String, (EntityKind, Span, bool)>>,
    pub id_gen: IdGenerator,
    pub importable_modules: &'a HashMap<String, TypeInfo>,

    pub imported_modules: HashMap<String, (Span, &'a TypeInfo)>,
    pub imported_types: HashSet<String>,

    pub module_types: HashMap<String, TypeConstructor>,

    pub module_types_constructors: HashMap<String, Vec<String>>,

    pub module_values: HashMap<String, ValueConstructor>,

    previous_id: u64,

    pub scope: HashMap<String, ValueConstructor>,

    pub ungeneralised_functions: HashSet<String>,

    pub unqualified_imported_names: HashMap<String, Span>,

    pub unused_modules: HashMap<String, Span>,

    pub warnings: &'a mut Vec<Warning>,
}

impl<'a> Environment<'a> {
    pub fn close_scope(&mut self, data: ScopeResetData) {
        let unused = self
            .entity_usages
            .pop()
            .expect("There was no top entity scope.");

        self.handle_unused(unused);

        self.scope = data.local_values;
    }

    pub fn convert_unused_to_warnings(&mut self) {
        let unused = self
            .entity_usages
            .pop()
            .expect("Expected a bottom level of entity usages.");

        self.handle_unused(unused);

        for (name, location) in self.unused_modules.clone().into_iter() {
            self.warnings
                .push(Warning::UnusedImportedModule { name, location });
        }
    }

    pub fn match_fun_type(
        &mut self,
        tipo: Arc<Type>,
        arity: usize,
        fn_location: Span,
        call_location: Span,
    ) -> Result<(Vec<Arc<Type>>, Arc<Type>), Error> {
        if let Type::Var { tipo } = tipo.deref() {
            let new_value = match tipo.borrow().deref() {
                TypeVar::Link { tipo, .. } => {
                    return self.match_fun_type(tipo.clone(), arity, fn_location, call_location);
                }

                TypeVar::Unbound { .. } => {
                    let args: Vec<_> = (0..arity).map(|_| self.new_unbound_var()).collect();

                    let ret = self.new_unbound_var();

                    Some((args, ret))
                }

                TypeVar::Generic { .. } => None,
            };

            if let Some((args, ret)) = new_value {
                *tipo.borrow_mut() = TypeVar::Link {
                    tipo: function(args.clone(), ret.clone()),
                };

                return Ok((args, ret));
            }
        }

        if let Type::Fn { args, ret } = tipo.deref() {
            return if args.len() != arity {
                Err(Error::IncorrectFunctionCallArity {
                    expected: args.len(),
                    given: arity,
                    location: call_location,
                })
            } else {
                Ok((args.clone(), ret.clone()))
            };
        }

        Err(Error::NotFn {
            tipo,
            location: fn_location,
        })
    }

    fn custom_type_accessors<A>(
        &mut self,
        constructors: &[RecordConstructor<A>],
        hydrator: &mut Hydrator,
    ) -> Result<Option<HashMap<String, RecordAccessor>>, Error> {
        let args = get_compatible_record_fields(constructors);

        let mut fields = HashMap::with_capacity(args.len());

        hydrator.disallow_new_type_variables();

        for (index, label, ast) in args {
            let tipo = hydrator.type_from_annotation(ast, self)?;

            fields.insert(
                label.to_string(),
                RecordAccessor {
                    index: index as u64,
                    label: label.to_string(),
                    tipo,
                },
            );
        }

        Ok(Some(fields))
    }

    pub fn generalise_definition(
        &mut self,
        s: TypedDefinition,
        module_name: &String,
    ) -> TypedDefinition {
        match s {
            Definition::Fn(Function {
                doc,
                location,
                name,
                public,
                arguments: args,
                body,
                return_annotation,
                return_type,
                end_position,
            }) => {
                let function = self
                    .get_variable(&name)
                    .expect("Could not find preregistered type for function");

                let field_map = function.field_map().cloned();

                let tipo = function.tipo.clone();

                let tipo = if self.ungeneralised_functions.remove(&name) {
                    generalise(tipo, 0)
                } else {
                    tipo
                };

                self.insert_module_value(
                    &name,
                    ValueConstructor {
                        public,
                        tipo,
                        variant: ValueConstructorVariant::ModuleFn {
                            name: name.clone(),
                            field_map,
                            module: module_name.to_owned(),
                            arity: args.len(),
                            location,
                            builtin: None,
                        },
                    },
                );

                Definition::Fn(Function {
                    doc,
                    location,
                    name,
                    public,
                    arguments: args,
                    return_annotation,
                    return_type,
                    body,
                    end_position,
                })
            }

            definition @ (Definition::TypeAlias { .. }
            | Definition::DataType { .. }
            | Definition::Use { .. }
            | Definition::Test { .. }
            | Definition::Validator { .. }
            | Definition::ModuleConstant { .. }) => definition,
        }
    }

    pub fn get_type_constructor(
        &mut self,
        module_alias: &Option<String>,
        name: &str,
        location: Span,
    ) -> Result<&TypeConstructor, Error> {
        match module_alias {
            None => self
                .module_types
                .get(name)
                .ok_or_else(|| Error::UnknownType {
                    location,
                    name: name.to_string(),
                    types: self.module_types.keys().map(|t| t.to_string()).collect(),
                }),

            Some(m) => {
                let (_, module) =
                    self.imported_modules
                        .get(m)
                        .ok_or_else(|| Error::UnknownModule {
                            location,
                            name: name.to_string(),
                            imported_modules: self
                                .importable_modules
                                .keys()
                                .map(|t| t.to_string())
                                .collect(),
                        })?;

                self.unused_modules.remove(m);

                module
                    .types
                    .get(name)
                    .ok_or_else(|| Error::UnknownModuleType {
                        location,
                        name: name.to_string(),
                        module_name: module.name.clone(),
                        type_constructors: module.types.keys().map(|t| t.to_string()).collect(),
                    })
            }
        }
    }

    pub fn get_value_constructor(
        &mut self,
        module: Option<&String>,
        name: &str,
        location: Span,
    ) -> Result<&ValueConstructor, Error> {
        match module {
            None => self.scope.get(name).ok_or_else(|| Error::UnknownVariable {
                location,
                name: name.to_string(),
                variables: self.local_value_names(),
            }),

            Some(m) => {
                let (_, module) =
                    self.imported_modules
                        .get(m)
                        .ok_or_else(|| Error::UnknownModule {
                            name: name.to_string(),
                            imported_modules: self
                                .importable_modules
                                .keys()
                                .map(|t| t.to_string())
                                .collect(),
                            location,
                        })?;

                self.unused_modules.remove(m);

                module
                    .values
                    .get(name)
                    .ok_or_else(|| Error::UnknownModuleValue {
                        name: name.to_string(),
                        module_name: module.name.clone(),
                        value_constructors: module.values.keys().map(|t| t.to_string()).collect(),
                        location,
                    })
            }
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<&ValueConstructor> {
        self.scope.get(name)
    }

    fn handle_unused(&mut self, unused: HashMap<String, (EntityKind, Span, bool)>) {
        for (name, (kind, location, _)) in unused.into_iter().filter(|(_, (_, _, used))| !used) {
            let warning = match kind {
                EntityKind::ImportedType | EntityKind::ImportedTypeAndConstructor => {
                    Warning::UnusedType {
                        name,
                        imported: true,
                        location,
                    }
                }
                EntityKind::ImportedConstructor => Warning::UnusedConstructor {
                    name,
                    imported: true,
                    location,
                },
                EntityKind::PrivateConstant => {
                    Warning::UnusedPrivateModuleConstant { name, location }
                }
                EntityKind::PrivateTypeConstructor(_) => Warning::UnusedConstructor {
                    name,
                    imported: false,
                    location,
                },
                EntityKind::PrivateFunction => Warning::UnusedPrivateFunction { name, location },
                EntityKind::PrivateType => Warning::UnusedType {
                    name,
                    imported: false,
                    location,
                },
                EntityKind::ImportedValue => Warning::UnusedImportedValue { name, location },
                EntityKind::Variable => Warning::UnusedVariable { name, location },
            };

            self.warnings.push(warning);
        }
    }

    pub fn in_new_scope<T>(&mut self, process_scope: impl FnOnce(&mut Self) -> T) -> T {
        let initial = self.open_new_scope();

        let result = process_scope(self);

        self.close_scope(initial);

        result
    }

    pub fn increment_usage(&mut self, name: &str) {
        let mut name = name.to_string();

        while let Some((kind, _, used)) = self
            .entity_usages
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(&name))
        {
            *used = true;

            match kind {
                EntityKind::PrivateTypeConstructor(type_name) if type_name != &name => {
                    name.clone_from(type_name);
                }
                _ => return,
            }
        }
    }

    pub fn init_usage(&mut self, name: String, kind: EntityKind, location: Span) {
        use EntityKind::*;

        match self
            .entity_usages
            .last_mut()
            .expect("Attempted to access non-existent entity usages scope")
            .insert(name.to_string(), (kind, location, false))
        {
            Some((ImportedType | ImportedTypeAndConstructor | PrivateType, _, _)) => (),

            Some((kind, location, false)) => {
                let mut unused = HashMap::with_capacity(1);
                unused.insert(name, (kind, location, false));
                self.handle_unused(unused);
            }

            _ => (),
        }
    }

    pub fn insert_accessors(&mut self, type_name: &str, accessors: AccessorsMap) {
        self.accessors.insert(type_name.to_string(), accessors);
    }

    pub fn insert_module_value(&mut self, name: &str, value: ValueConstructor) {
        self.module_values.insert(name.to_string(), value);
    }

    pub fn insert_type_constructor(
        &mut self,
        type_name: String,
        info: TypeConstructor,
    ) -> Result<(), Error> {
        let name = type_name.clone();
        let location = info.location;

        match self.module_types.insert(type_name, info) {
            None => Ok(()),
            Some(prelude_type) if prelude_type.module.is_empty() => Ok(()),
            Some(previous) => Err(Error::DuplicateTypeName {
                name,
                location,
                previous_location: previous.location,
            }),
        }
    }

    pub fn insert_type_to_constructors(&mut self, type_name: String, constructors: Vec<String>) {
        self.module_types_constructors
            .insert(type_name, constructors);
    }

    pub fn insert_variable(
        &mut self,
        name: String,
        variant: ValueConstructorVariant,
        tipo: Arc<Type>,
    ) {
        self.scope.insert(
            name,
            ValueConstructor {
                public: false,
                variant,
                tipo,
            },
        );
    }

    pub fn instantiate(
        &mut self,
        t: Arc<Type>,
        ids: &mut HashMap<u64, Arc<Type>>,
        hydrator: &Hydrator,
    ) -> Arc<Type> {
        match t.deref() {
            Type::App {
                public,
                name,
                module,
                args,
            } => {
                let args = args
                    .iter()
                    .map(|t| self.instantiate(t.clone(), ids, hydrator))
                    .collect();
                Arc::new(Type::App {
                    public: *public,
                    name: name.clone(),
                    module: module.clone(),
                    args,
                })
            }

            Type::Var { tipo } => {
                match tipo.borrow().deref() {
                    TypeVar::Link { tipo } => return self.instantiate(tipo.clone(), ids, hydrator),

                    TypeVar::Unbound { .. } => return Arc::new(Type::Var { tipo: tipo.clone() }),

                    TypeVar::Generic { id } => match ids.get(id) {
                        Some(t) => return t.clone(),
                        None => {
                            if !hydrator.is_rigid(id) {
                                let v = self.new_unbound_var();
                                ids.insert(*id, v.clone());
                                return v;
                            } else {
                            }
                        }
                    },
                }
                Arc::new(Type::Var { tipo: tipo.clone() })
            }

            Type::Fn { args, ret, .. } => function(
                args.iter()
                    .map(|t| self.instantiate(t.clone(), ids, hydrator))
                    .collect(),
                self.instantiate(ret.clone(), ids, hydrator),
            ),

            Type::Tuple { elems } => tuple(
                elems
                    .iter()
                    .map(|t| self.instantiate(t.clone(), ids, hydrator))
                    .collect(),
            ),
        }
    }

    pub fn local_value_names(&self) -> Vec<String> {
        self.scope
            .keys()
            .filter(|&t| PIPE_VARIABLE != t)
            .map(|t| t.to_string())
            .collect()
    }

    fn make_type_vars(
        &mut self,
        args: &[String],
        location: &Span,
        hydrator: &mut Hydrator,
    ) -> Result<Vec<Arc<Type>>, Error> {
        let mut type_vars = Vec::new();

        for arg in args {
            let annotation = Annotation::Var {
                location: *location,
                name: arg.to_string(),
            };

            let tipo = hydrator.type_from_annotation(&annotation, self)?;

            type_vars.push(tipo);
        }

        Ok(type_vars)
    }

    pub fn new(
        id_gen: IdGenerator,
        current_module: &'a String,
        importable_modules: &'a HashMap<String, TypeInfo>,
        warnings: &'a mut Vec<Warning>,
    ) -> Self {
        let prelude = importable_modules
            .get("nano")
            .expect("Unable to find prelude in importable modules");

        Self {
            previous_id: id_gen.next(),
            id_gen,
            ungeneralised_functions: HashSet::new(),
            module_types: prelude.types.clone(),
            module_types_constructors: prelude.types_constructors.clone(),
            module_values: HashMap::new(),
            imported_modules: HashMap::new(),
            unused_modules: HashMap::new(),
            unqualified_imported_names: HashMap::new(),
            accessors: prelude.accessors.clone(),
            scope: prelude.values.clone(),
            importable_modules,
            imported_types: HashSet::new(),
            current_module,
            warnings,
            entity_usages: vec![HashMap::new()],
        }
    }

    pub fn new_generic_var(&mut self) -> Arc<Type> {
        generic_var(self.next_uid())
    }

    pub fn new_unbound_var(&mut self) -> Arc<Type> {
        unbound_var(self.next_uid())
    }

    pub fn next_uid(&mut self) -> u64 {
        let id = self.id_gen.next();
        self.previous_id = id;
        id
    }

    pub fn open_new_scope(&mut self) -> ScopeResetData {
        let local_values = self.scope.clone();

        self.entity_usages.push(HashMap::new());

        ScopeResetData { local_values }
    }

    pub fn previous_uid(&self) -> u64 {
        self.previous_id
    }

    pub fn register_import(&mut self, def: &UntypedDefinition) -> Result<(), Error> {
        match def {
            Definition::Use(Use {
                module,
                as_name,
                unqualified,
                location,
                ..
            }) => {
                let name = module.join("/");

                let module_info =
                    self.importable_modules
                        .get(&name)
                        .ok_or_else(|| Error::UnknownModule {
                            location: *location,
                            name: name.clone(),
                            imported_modules: self.imported_modules.keys().cloned().collect(),
                        })?;

                if module_info.kind.is_validator() {
                    return Err(Error::ValidatorImported {
                        location: *location,
                        name,
                    });
                }

                let module_name = as_name
                    .as_ref()
                    .or_else(|| module.last())
                    .expect("Typer could not identify module name.")
                    .clone();

                for UnqualifiedImport {
                    name,
                    location,
                    as_name,
                    ..
                } in unqualified
                {
                    let mut type_imported = false;
                    let mut value_imported = false;
                    let mut variant = None;

                    let imported_name = as_name.as_ref().unwrap_or(name);

                    if let Some(previous) = self.unqualified_imported_names.get(imported_name) {
                        return Err(Error::DuplicateImport {
                            location: *location,
                            previous_location: *previous,
                            name: name.to_string(),
                            module: module.clone(),
                        });
                    }

                    self.unqualified_imported_names
                        .insert(imported_name.clone(), *location);

                    if let Some(value) = module_info.values.get(name) {
                        self.insert_variable(
                            imported_name.clone(),
                            value.variant.clone(),
                            value.tipo.clone(),
                        );
                        variant = Some(&value.variant);
                        value_imported = true;
                    }

                    if let Some(typ) = module_info.types.get(name) {
                        let typ_info = TypeConstructor {
                            location: *location,
                            ..typ.clone()
                        };

                        self.insert_type_constructor(imported_name.clone(), typ_info)?;

                        type_imported = true;
                    }

                    if value_imported && type_imported {
                        self.init_usage(
                            imported_name.to_string(),
                            EntityKind::ImportedTypeAndConstructor,
                            *location,
                        );
                    } else if type_imported {
                        self.imported_types.insert(imported_name.to_string());

                        self.init_usage(
                            imported_name.to_string(),
                            EntityKind::ImportedType,
                            *location,
                        );
                    } else if value_imported {
                        match variant {
                            Some(&ValueConstructorVariant::Record { .. }) => self.init_usage(
                                imported_name.to_string(),
                                EntityKind::ImportedConstructor,
                                *location,
                            ),
                            _ => self.init_usage(
                                imported_name.to_string(),
                                EntityKind::ImportedValue,
                                *location,
                            ),
                        };
                    } else if !value_imported {
                        return Err(Error::UnknownModuleField {
                            location: *location,
                            name: name.clone(),
                            module_name: module.join("/"),
                            value_constructors: module_info
                                .values
                                .keys()
                                .map(|t| t.to_string())
                                .collect(),
                            type_constructors: module_info
                                .types
                                .keys()
                                .map(|t| t.to_string())
                                .collect(),
                        });
                    }
                }

                if unqualified.is_empty() {
                    self.unused_modules.insert(module_name.clone(), *location);
                }

                if let Some((previous_location, _)) = self.imported_modules.get(&module_name) {
                    return Err(Error::DuplicateImport {
                        location: *location,
                        previous_location: *previous_location,
                        name: module_name,
                        module: module.clone(),
                    });
                }

                self.unqualified_imported_names
                    .insert(module_name.clone(), *location);

                self.imported_modules
                    .insert(module_name, (*location, module_info));

                Ok(())
            }

            _ => Ok(()),
        }
    }

    pub fn register_types(
        &mut self,
        definitions: Vec<&'a UntypedDefinition>,
        module: &String,
        hydrators: &mut HashMap<String, Hydrator>,
        names: &mut HashMap<&'a str, &'a Span>,
    ) -> Result<(), Error> {
        let known_types_before = names.keys().copied().collect::<Vec<_>>();

        let mut error = None;
        let mut remaining_definitions = vec![];

        for def in definitions {
            if let Err(e) = self.register_type(def, module, hydrators, names) {
                error = Some(e);
                remaining_definitions.push(def);
                if let Definition::TypeAlias(TypeAlias { alias, .. }) = def {
                    names.remove(alias.as_str());
                }
            };
        }

        match error {
            None => Ok(()),
            Some(e) => {
                let known_types_after = names.keys().copied().collect::<Vec<_>>();
                if known_types_before == known_types_after {
                    let unknown_name = match e {
                        Error::UnknownType { ref name, .. } => name,
                        _ => "",
                    };
                    let mut is_cyclic = false;
                    let unknown_types = remaining_definitions
                        .into_iter()
                        .filter_map(|def| match def {
                            Definition::TypeAlias(TypeAlias {
                                alias, location, ..
                            }) => {
                                is_cyclic = is_cyclic || alias == unknown_name;
                                Some(Snippet {
                                    location: location.to_owned(),
                                })
                            }
                            Definition::DataType(DataType { name, location, .. }) => {
                                is_cyclic = is_cyclic || name == unknown_name;
                                Some(Snippet {
                                    location: location.to_owned(),
                                })
                            }
                            Definition::Fn { .. }
                            | Definition::Validator { .. }
                            | Definition::Use { .. }
                            | Definition::ModuleConstant { .. }
                            | Definition::Test { .. } => None,
                        })
                        .collect::<Vec<Snippet>>();

                    if is_cyclic {
                        Err(Error::CyclicTypeDefinitions {
                            errors: unknown_types,
                        })
                    } else {
                        Err(e)
                    }
                } else {
                    self.register_types(remaining_definitions, module, hydrators, names)
                }
            }
        }
    }

    pub fn register_type(
        &mut self,
        def: &'a UntypedDefinition,
        module: &String,
        hydrators: &mut HashMap<String, Hydrator>,
        names: &mut HashMap<&'a str, &'a Span>,
    ) -> Result<(), Error> {
        match def {
            Definition::DataType(DataType {
                name,
                public,
                parameters,
                location,
                constructors,
                ..
            }) => {
                assert_unique_type_name(names, name, location)?;

                let mut hydrator = Hydrator::new();

                let parameters = self.make_type_vars(parameters, location, &mut hydrator)?;

                let tipo = Arc::new(Type::App {
                    public: *public,
                    module: module.to_owned(),
                    name: name.clone(),
                    args: parameters.clone(),
                });

                hydrators.insert(name.to_string(), hydrator);

                self.insert_type_constructor(
                    name.clone(),
                    TypeConstructor {
                        location: *location,
                        module: module.to_owned(),
                        public: *public,
                        parameters,
                        tipo,
                    },
                )?;

                let constructor_names = constructors.iter().map(|c| c.name.clone()).collect();

                self.insert_type_to_constructors(name.clone(), constructor_names);

                if !public {
                    self.init_usage(name.clone(), EntityKind::PrivateType, *location);
                }
            }

            Definition::TypeAlias(TypeAlias {
                location,
                public,
                parameters: args,
                alias: name,
                annotation: resolved_type,
                ..
            }) => {
                assert_unique_type_name(names, name, location)?;

                let mut hydrator = Hydrator::new();
                let parameters = self.make_type_vars(args, location, &mut hydrator)?;

                hydrator.disallow_new_type_variables();

                let tipo = hydrator.type_from_annotation(resolved_type, self)?;

                self.insert_type_constructor(
                    name.clone(),
                    TypeConstructor {
                        location: *location,
                        module: module.to_owned(),
                        public: *public,
                        parameters,
                        tipo,
                    },
                )?;

                if !public {
                    self.init_usage(name.clone(), EntityKind::PrivateType, *location);
                }
            }

            Definition::Fn { .. }
            | Definition::Validator { .. }
            | Definition::Test { .. }
            | Definition::Use { .. }
            | Definition::ModuleConstant { .. } => {}
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn register_function(
        &mut self,
        name: &'a str,
        arguments: &[UntypedArg],
        return_annotation: &Option<Annotation>,
        module_name: &String,
        hydrators: &mut HashMap<String, Hydrator>,
        names: &mut HashMap<&'a str, &'a Span>,
        location: &'a Span,
    ) -> Result<(), Error> {
        assert_unique_value_name(names, name, location)?;

        self.ungeneralised_functions.insert(name.to_string());

        let mut field_map = FieldMap::new(arguments.len(), true);

        for (i, arg) in arguments.iter().enumerate() {
            field_map.insert(arg.arg_name.get_label().clone(), i, &arg.location)?;
        }
        let field_map = field_map.into_option();

        let mut hydrator = Hydrator::new();

        let mut arg_types = Vec::new();

        for arg in arguments {
            let tipo = hydrator.type_from_option_annotation(&arg.annotation, self)?;

            arg_types.push(tipo);
        }

        let return_type = hydrator.type_from_option_annotation(return_annotation, self)?;

        let tipo = function(arg_types, return_type);

        hydrators.insert(name.to_string(), hydrator);

        self.insert_variable(
            name.to_string(),
            ValueConstructorVariant::ModuleFn {
                name: name.to_string(),
                field_map,
                module: module_name.to_owned(),
                arity: arguments.len(),
                location: *location,
                builtin: None,
            },
            tipo,
        );

        Ok(())
    }

    pub fn register_values(
        &mut self,
        def: &'a UntypedDefinition,
        module_name: &String,
        hydrators: &mut HashMap<String, Hydrator>,
        names: &mut HashMap<&'a str, &'a Span>,
        kind: ModuleKind,
    ) -> Result<(), Error> {
        match def {
            Definition::Fn(fun) => {
                self.register_function(
                    &fun.name,
                    &fun.arguments,
                    &fun.return_annotation,
                    module_name,
                    hydrators,
                    names,
                    &fun.location,
                )?;

                if !fun.public && kind.is_lib() {
                    self.init_usage(fun.name.clone(), EntityKind::PrivateFunction, fun.location);
                }
            }

            Definition::Validator(Validator {
                fun,
                other_fun,
                params,
                ..
            }) if kind.is_validator() => {
                let temp_params: Vec<UntypedArg> = params
                    .iter()
                    .cloned()
                    .chain(fun.arguments.clone())
                    .collect();

                self.register_function(
                    &fun.name,
                    &temp_params,
                    &fun.return_annotation,
                    module_name,
                    hydrators,
                    names,
                    &fun.location,
                )?;

                if let Some(other) = other_fun {
                    let temp_params: Vec<UntypedArg> = params
                        .iter()
                        .cloned()
                        .chain(other.arguments.clone())
                        .collect();

                    self.register_function(
                        &other.name,
                        &temp_params,
                        &other.return_annotation,
                        module_name,
                        hydrators,
                        names,
                        &other.location,
                    )?;
                }
            }

            Definition::Validator(Validator { location, .. }) => {
                self.warnings.push(Warning::ValidatorInLibraryModule {
                    location: *location,
                })
            }

            Definition::Test(Function { name, location, .. }) => {
                assert_unique_value_name(names, name, location)?;
                hydrators.insert(name.clone(), Hydrator::new());
                let arg_types = vec![];
                let return_type = builtins::bool();
                self.insert_variable(
                    name.clone(),
                    ValueConstructorVariant::ModuleFn {
                        name: name.clone(),
                        field_map: None,
                        module: module_name.to_owned(),
                        arity: 0,
                        location: *location,
                        builtin: None,
                    },
                    function(arg_types, return_type),
                );
            }

            Definition::DataType(DataType {
                public,
                opaque,
                name,
                constructors,
                ..
            }) => {
                let mut hydrator = hydrators
                    .remove(name)
                    .expect("Could not find hydrator for register_values custom type");

                hydrator.disallow_new_type_variables();

                let typ = self
                    .module_types
                    .get(name)
                    .expect("Type for custom type not found in register_values")
                    .tipo
                    .clone();

                if let Some(accessors) = self.custom_type_accessors(constructors, &mut hydrator)? {
                    let map = AccessorsMap {
                        public: (*public && !*opaque),
                        accessors,
                        tipo: typ.clone(),
                    };

                    self.insert_accessors(name, map)
                }

                for constructor in constructors {
                    assert_unique_value_name(names, &constructor.name, &constructor.location)?;

                    let mut field_map = FieldMap::new(constructor.arguments.len(), false);

                    let mut args_types = Vec::with_capacity(constructor.arguments.len());

                    for (
                        i,
                        RecordConstructorArg {
                            label,
                            annotation,
                            location,
                            ..
                        },
                    ) in constructor.arguments.iter().enumerate()
                    {
                        let t = hydrator.type_from_annotation(annotation, self)?;

                        args_types.push(t);

                        if let Some(label) = label {
                            field_map.insert(label.clone(), i, location)?;
                        }
                    }

                    let field_map = field_map.into_option();

                    let typ = match constructor.arguments.len() {
                        0 => typ.clone(),
                        _ => function(args_types, typ.clone()),
                    };

                    let constructor_info = ValueConstructorVariant::Record {
                        constructors_count: constructors.len() as u16,
                        name: constructor.name.clone(),
                        arity: constructor.arguments.len(),
                        field_map: field_map.clone(),
                        location: constructor.location,
                        module: module_name.to_owned(),
                    };

                    if !opaque {
                        self.insert_module_value(
                            &constructor.name,
                            ValueConstructor {
                                public: *public,
                                tipo: typ.clone(),
                                variant: constructor_info.clone(),
                            },
                        );
                    }

                    if !public {
                        self.init_usage(
                            constructor.name.clone(),
                            EntityKind::PrivateTypeConstructor(name.clone()),
                            constructor.location,
                        );
                    }

                    self.insert_variable(constructor.name.clone(), constructor_info, typ);
                }
            }

            Definition::ModuleConstant(ModuleConstant { name, location, .. }) => {
                assert_unique_const_name(names, name, location)?;
            }

            Definition::Use { .. } | Definition::TypeAlias { .. } => {}
        }
        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn unify(
        &mut self,
        t1: Arc<Type>,
        t2: Arc<Type>,
        location: Span,
        allow_cast: bool,
    ) -> Result<(), Error> {
        if t1 == t2 {
            return Ok(());
        }

        if allow_cast
            && (t1.is_data() || t2.is_data())
            && !(t1.is_unbound() || t2.is_unbound())
            && !(t1.is_function() || t2.is_function())
            && !(t1.is_generic() || t2.is_generic())
            && !(t1.is_string() || t2.is_string())
        {
            return Ok(());
        }

        if let Type::Var { tipo } = t2.deref() {
            if let TypeVar::Link { tipo } = tipo.borrow().deref() {
                return self.unify(t1, tipo.clone(), location, allow_cast);
            }
        }

        if let Type::Var { tipo } = t1.deref() {
            enum Action {
                Unify(Arc<Type>),
                CouldNotUnify,
                Link,
            }

            let action = match tipo.borrow().deref() {
                TypeVar::Link { tipo } => Action::Unify(tipo.clone()),

                TypeVar::Unbound { id } => {
                    unify_unbound_type(t2.clone(), *id, location)?;
                    Action::Link
                }

                TypeVar::Generic { id } => {
                    if let Type::Var { tipo } = t2.deref() {
                        if tipo.borrow().is_unbound() {
                            *tipo.borrow_mut() = TypeVar::Generic { id: *id };
                            return Ok(());
                        }
                    }
                    Action::CouldNotUnify
                }
            };

            return match action {
                Action::Link => {
                    *tipo.borrow_mut() = TypeVar::Link { tipo: t2 };
                    Ok(())
                }

                Action::Unify(t) => self.unify(t, t2, location, allow_cast),

                Action::CouldNotUnify => Err(Error::CouldNotUnify {
                    location,
                    expected: t1.clone(),
                    given: t2,
                    situation: None,
                    rigid_type_names: HashMap::new(),
                }),
            };
        }

        if let Type::Var { .. } = t2.deref() {
            return self
                .unify(t2, t1, location, allow_cast)
                .map_err(|e| e.flip_unify());
        }

        match (t1.deref(), t2.deref()) {
            (
                Type::App {
                    module: m1,
                    name: n1,
                    args: args1,
                    ..
                },
                Type::App {
                    module: m2,
                    name: n2,
                    args: args2,
                    ..
                },
            ) if m1 == m2 && n1 == n2 && args1.len() == args2.len() => {
                for (a, b) in args1.iter().zip(args2) {
                    unify_enclosed_type(
                        t1.clone(),
                        t2.clone(),
                        self.unify(a.clone(), b.clone(), location, allow_cast),
                    )?;
                }
                Ok(())
            }

            (Type::Tuple { elems: elems1, .. }, Type::Tuple { elems: elems2, .. })
                if elems1.len() == elems2.len() =>
            {
                for (a, b) in elems1.iter().zip(elems2) {
                    unify_enclosed_type(
                        t1.clone(),
                        t2.clone(),
                        self.unify(a.clone(), b.clone(), location, allow_cast),
                    )?;
                }
                Ok(())
            }

            (
                Type::Fn {
                    args: args1,
                    ret: retrn1,
                    ..
                },
                Type::Fn {
                    args: args2,
                    ret: retrn2,
                    ..
                },
            ) if args1.len() == args2.len() => {
                for (a, b) in args1.iter().zip(args2) {
                    self.unify(a.clone(), b.clone(), location, allow_cast)
                        .map_err(|_| Error::CouldNotUnify {
                            location,
                            expected: t1.clone(),
                            given: t2.clone(),
                            situation: None,
                            rigid_type_names: HashMap::new(),
                        })?;
                }
                self.unify(retrn1.clone(), retrn2.clone(), location, allow_cast)
                    .map_err(|_| Error::CouldNotUnify {
                        location,
                        expected: t1.clone(),
                        given: t2.clone(),
                        situation: None,
                        rigid_type_names: HashMap::new(),
                    })
            }

            _ => Err(Error::CouldNotUnify {
                location,
                expected: t1.clone(),
                given: t2.clone(),
                situation: None,
                rigid_type_names: HashMap::new(),
            }),
        }
    }

    pub fn check_exhaustiveness(
        &mut self,
        patterns: Vec<Pattern<PatternConstructor, Arc<Type>>>,
        value_typ: Arc<Type>,
        location: Span,
    ) -> Result<(), Vec<String>> {
        match &*value_typ {
            Type::App {
                name: type_name,
                module,
                ..
            } => {
                let m = if module.is_empty() || module == self.current_module {
                    None
                } else {
                    Some(module.clone())
                };

                if type_name == "List" && module.is_empty() {
                    return self.check_list_pattern_exhaustiveness(patterns);
                }

                if let Ok(constructors) = self.get_constructors_for_type(&m, type_name, location) {
                    let mut unmatched_constructors: HashSet<String> =
                        constructors.iter().cloned().collect();

                    for p in &patterns {
                        let mut pattern = p;
                        while let Pattern::Assign {
                            pattern: assign_pattern,
                            ..
                        } = pattern
                        {
                            pattern = assign_pattern;
                        }

                        match pattern {
                            Pattern::Discard { .. } => return Ok(()),
                            Pattern::Var { .. } => return Ok(()),
                            Pattern::Constructor {
                                constructor: PatternConstructor::Record { name, .. },
                                ..
                            } => {
                                unmatched_constructors.remove(name);
                            }
                            _ => return Ok(()),
                        }
                    }

                    if !unmatched_constructors.is_empty() {
                        return Err(unmatched_constructors.into_iter().sorted().collect());
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn check_list_pattern_exhaustiveness(
        &mut self,
        patterns: Vec<Pattern<PatternConstructor, Arc<Type>>>,
    ) -> Result<(), Vec<String>> {
        let mut cover_empty = false;
        let mut cover_tail = false;

        let patterns = patterns.iter().map(|p| match p {
            Pattern::Assign { pattern, .. } => pattern,
            _ => p,
        });

        for p in patterns {
            match p {
                Pattern::Var { .. } => {
                    cover_empty = true;
                    cover_tail = true;
                }
                Pattern::Discard { .. } => {
                    cover_empty = true;
                    cover_tail = true;
                }
                Pattern::List { elements, tail, .. } => {
                    if elements.is_empty() {
                        cover_empty = true;
                    }
                    match tail {
                        None => {}
                        Some(p) => match **p {
                            Pattern::Discard { .. } => {
                                cover_tail = true;
                            }
                            Pattern::Var { .. } => {
                                cover_tail = true;
                            }
                            _ => {
                                unreachable!()
                            }
                        },
                    }
                }
                _ => {}
            }
        }

        if cover_empty && cover_tail {
            Ok(())
        } else {
            let mut missing = vec![];
            if !cover_empty {
                missing.push("[]".to_owned());
            }
            if !cover_tail {
                missing.push("[_, ..]".to_owned());
            }
            Err(missing)
        }
    }

    pub fn get_constructors_for_type(
        &mut self,
        full_module_name: &Option<String>,
        name: &str,
        location: Span,
    ) -> Result<&Vec<String>, Error> {
        match full_module_name {
            None => self
                .module_types_constructors
                .get(name)
                .ok_or_else(|| Error::UnknownType {
                    name: name.to_string(),
                    types: self.module_types.keys().map(|t| t.to_string()).collect(),
                    location,
                }),

            Some(m) => {
                let module =
                    self.importable_modules
                        .get(m)
                        .ok_or_else(|| Error::UnknownModule {
                            location,
                            name: name.to_string(),
                            imported_modules: self
                                .importable_modules
                                .keys()
                                .map(|t| t.to_string())
                                .collect(),
                        })?;

                self.unused_modules.remove(m);

                module
                    .types_constructors
                    .get(name)
                    .ok_or_else(|| Error::UnknownModuleType {
                        location,
                        name: name.to_string(),
                        module_name: module.name.clone(),
                        type_constructors: module.types.keys().map(|t| t.to_string()).collect(),
                    })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityKind {
    PrivateConstant,
    PrivateTypeConstructor(String),
    PrivateFunction,
    ImportedConstructor,
    ImportedType,
    ImportedTypeAndConstructor,
    ImportedValue,
    PrivateType,
    Variable,
}

fn unify_unbound_type(tipo: Arc<Type>, own_id: u64, location: Span) -> Result<(), Error> {
    if let Type::Var { tipo } = tipo.deref() {
        let new_value = match tipo.borrow().deref() {
            TypeVar::Link { tipo, .. } => {
                return unify_unbound_type(tipo.clone(), own_id, location)
            }

            TypeVar::Unbound { id } => {
                if id == &own_id {
                    return Err(Error::RecursiveType { location });
                } else {
                    Some(TypeVar::Unbound { id: *id })
                }
            }

            TypeVar::Generic { .. } => return Ok(()),
        };

        if let Some(t) = new_value {
            *tipo.borrow_mut() = t;
        }
        return Ok(());
    }

    match tipo.deref() {
        Type::App { args, .. } => {
            for arg in args {
                unify_unbound_type(arg.clone(), own_id, location)?
            }

            Ok(())
        }

        Type::Fn { args, ret } => {
            for arg in args {
                unify_unbound_type(arg.clone(), own_id, location)?;
            }

            unify_unbound_type(ret.clone(), own_id, location)
        }

        Type::Tuple { elems, .. } => {
            for elem in elems {
                unify_unbound_type(elem.clone(), own_id, location)?
            }

            Ok(())
        }

        Type::Var { .. } => unreachable!(),
    }
}

fn unify_enclosed_type(
    e1: Arc<Type>,
    e2: Arc<Type>,
    result: Result<(), Error>,
) -> Result<(), Error> {
    match result {
        Err(Error::CouldNotUnify {
            situation,
            location,
            rigid_type_names,
            ..
        }) => Err(Error::CouldNotUnify {
            expected: e1,
            given: e2,
            situation,
            location,
            rigid_type_names,
        }),

        _ => result,
    }
}

fn assert_unique_type_name<'a>(
    names: &mut HashMap<&'a str, &'a Span>,
    name: &'a str,
    location: &'a Span,
) -> Result<(), Error> {
    match names.insert(name, location) {
        Some(previous_location) => Err(Error::DuplicateTypeName {
            name: name.to_string(),
            previous_location: *previous_location,
            location: *location,
        }),
        None => Ok(()),
    }
}

fn assert_unique_value_name<'a>(
    names: &mut HashMap<&'a str, &'a Span>,
    name: &'a str,
    location: &'a Span,
) -> Result<(), Error> {
    match names.insert(name, location) {
        Some(previous_location) => Err(Error::DuplicateName {
            name: name.to_string(),
            previous_location: *previous_location,
            location: *location,
        }),
        None => Ok(()),
    }
}

fn assert_unique_const_name<'a>(
    names: &mut HashMap<&'a str, &'a Span>,
    name: &'a str,
    location: &'a Span,
) -> Result<(), Error> {
    match names.insert(name, location) {
        Some(previous_location) => Err(Error::DuplicateConstName {
            name: name.to_string(),
            previous_location: *previous_location,
            location: *location,
        }),
        None => Ok(()),
    }
}

pub(super) fn assert_no_labeled_arguments<A>(args: &[CallArg<A>]) -> Option<(Span, String)> {
    for arg in args {
        if let Some(label) = &arg.label {
            return Some((arg.location, label.to_string()));
        }
    }
    None
}

pub(super) fn collapse_links(t: Arc<Type>) -> Arc<Type> {
    if let Type::Var { tipo } = t.deref() {
        if let TypeVar::Link { tipo } = tipo.borrow().deref() {
            return tipo.clone();
        }
    }
    t
}

fn get_compatible_record_fields<A>(
    constructors: &[RecordConstructor<A>],
) -> Vec<(usize, &str, &Annotation)> {
    let mut compatible = vec![];

    if constructors.len() > 1 {
        return compatible;
    }

    let first = match constructors.get(0) {
        Some(first) => first,
        None => return compatible,
    };

    for (index, first_argument) in first.arguments.iter().enumerate() {
        let label = match first_argument.label.as_ref() {
            Some(label) => label.as_str(),
            None => continue,
        };

        compatible.push((index, label, &first_argument.annotation))
    }

    compatible
}

#[allow(clippy::only_used_in_recursion)]
pub(crate) fn generalise(t: Arc<Type>, ctx_level: usize) -> Arc<Type> {
    match t.deref() {
        Type::Var { tipo } => match tipo.borrow().deref() {
            TypeVar::Unbound { id } => generic_var(*id),
            TypeVar::Link { tipo } => generalise(tipo.clone(), ctx_level),
            TypeVar::Generic { .. } => Arc::new(Type::Var { tipo: tipo.clone() }),
        },

        Type::App {
            public,
            module,
            name,
            args,
        } => {
            let args = args
                .iter()
                .map(|t| generalise(t.clone(), ctx_level))
                .collect();

            Arc::new(Type::App {
                public: *public,
                module: module.clone(),
                name: name.clone(),
                args,
            })
        }

        Type::Fn { args, ret } => function(
            args.iter()
                .map(|t| generalise(t.clone(), ctx_level))
                .collect(),
            generalise(ret.clone(), ctx_level),
        ),

        Type::Tuple { elems } => tuple(
            elems
                .iter()
                .map(|t| generalise(t.clone(), ctx_level))
                .collect(),
        ),
    }
}