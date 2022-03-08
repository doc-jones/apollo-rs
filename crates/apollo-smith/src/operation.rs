use std::collections::HashMap;

use arbitrary::{Arbitrary, Result};

use crate::{
    directive::{Directive, DirectiveLocation},
    name::Name,
    selection_set::SelectionSet,
    variable::VariableDef,
    DocumentBuilder,
};

/// The __operationDef type represents an operation definition
///
/// *OperationDefinition*:
///     OperationType Name? VariableDefinitions? Directives? SelectionSet
///
/// Detailed documentation can be found in [GraphQL spec](https://spec.graphql.org/October2021/#sec-Language.Operations).
#[derive(Debug)]
pub struct OperationDef {
    pub(crate) operation_type: OperationType,
    pub(crate) name: Option<Name>,
    pub(crate) variable_definitions: Vec<VariableDef>,
    pub(crate) directives: HashMap<Name, Directive>,
    pub(crate) selection_set: SelectionSet,
    pub(crate) shorthand: bool,
}

impl From<OperationDef> for apollo_encoder::OperationDefinition {
    fn from(op_def: OperationDef) -> Self {
        let mut new_op_def = Self::new(op_def.operation_type.into(), op_def.selection_set.into());
        new_op_def.name(op_def.name.map(String::from));
        op_def
            .variable_definitions
            .into_iter()
            .for_each(|var_def| new_op_def.variable_definition(var_def.into()));
        op_def.shorthand.then(|| new_op_def.shorthand());
        op_def
            .directives
            .into_iter()
            .for_each(|(_, directive)| new_op_def.directive(directive.into()));

        new_op_def
    }
}

impl From<OperationDef> for String {
    fn from(op_def: OperationDef) -> Self {
        apollo_encoder::OperationDefinition::from(op_def).to_string()
    }
}

impl From<apollo_parser::ast::OperationDefinition> for OperationDef {
    fn from(operation_def: apollo_parser::ast::OperationDefinition) -> Self {
        Self {
            name: operation_def.name().map(Name::from),
            directives: operation_def
                .directives()
                .map(|d| {
                    d.directives()
                        .map(|d| (d.name().unwrap().into(), Directive::from(d)))
                        .collect()
                })
                .unwrap_or_default(),
            operation_type: operation_def
                .operation_type()
                .map(OperationType::from)
                .unwrap_or(OperationType::Query),
            variable_definitions: Vec::new(),
            selection_set: operation_def.selection_set().unwrap().into(),
            shorthand: operation_def.operation_type().is_none(),
        }
    }
}

/// The __operationType type represents the kind of operation
///
/// *OperationType*:
///     query | mutation | subscription
///
/// Detailed documentation can be found in [GraphQL spec](https://spec.graphql.org/October2021/#OperationType).
#[derive(Debug, Arbitrary, Clone, Copy, PartialEq)]
pub enum OperationType {
    Query,
    Mutation,
    Subscription,
}

impl From<OperationType> for apollo_encoder::OperationType {
    fn from(op_type: OperationType) -> Self {
        match op_type {
            OperationType::Query => Self::Query,
            OperationType::Mutation => Self::Mutation,
            OperationType::Subscription => Self::Subscription,
        }
    }
}

impl From<apollo_parser::ast::OperationType> for OperationType {
    fn from(op_type: apollo_parser::ast::OperationType) -> Self {
        if op_type.query_token().is_some() {
            Self::Query
        } else if op_type.mutation_token().is_some() {
            Self::Mutation
        } else if op_type.subscription_token().is_some() {
            Self::Subscription
        } else {
            Self::Query
        }
    }
}

impl<'a> DocumentBuilder<'a> {
    /// Create an arbitrary `OperationDef`
    pub fn operation_definition(&mut self) -> Result<OperationDef> {
        let name = self
            .u
            .arbitrary()
            .unwrap_or(false)
            .then(|| self.type_name())
            .transpose()?;

        let operation_type = self.u.arbitrary()?;
        let directive_location = match operation_type {
            OperationType::Query => DirectiveLocation::Query,
            OperationType::Mutation => DirectiveLocation::Mutation,
            OperationType::Subscription => DirectiveLocation::Subscription,
        };
        let directives = self.directives(directive_location)?;
        let selection_set = self.selection_set()?;
        let variable_definitions = self.variable_definitions()?;
        let shorthand = self.operation_defs.is_empty()
            && operation_type == OperationType::Query
            && self.u.arbitrary().unwrap_or(false);

        Ok(OperationDef {
            operation_type,
            name,
            variable_definitions,
            directives,
            selection_set,
            shorthand,
        })
    }

    /// Create an arbitrary `OperationDef` given a `SchemaDef`
    pub fn operation_definition_from_schema(&mut self) -> Result<OperationDef> {
        let schema = self.schema_defs.last().cloned().unwrap();
        let name = self
            .u
            .arbitrary()
            .unwrap_or(false)
            .then(|| self.type_name())
            .transpose()?;
        let available_operations = {
            let mut ops = vec![];
            if let Some(query) = &schema.query {
                ops.push((OperationType::Query, query));
            }
            if let Some(mutation) = &schema.mutation {
                ops.push((OperationType::Mutation, mutation));
            }
            if let Some(subscription) = &schema.subscription {
                ops.push((OperationType::Subscription, subscription));
            }

            ops
        };
        let (operation_type, choosen_ty) = self.u.choose(&available_operations)?;
        let directive_location = match operation_type {
            OperationType::Query => DirectiveLocation::Query,
            OperationType::Mutation => DirectiveLocation::Mutation,
            OperationType::Subscription => DirectiveLocation::Subscription,
        };
        let directives = self.directives(directive_location)?;

        // Stack
        self.stack_ty(choosen_ty);

        let selection_set = self.selection_set()?;

        self.stack.pop();

        // TODO
        let variable_definitions = vec![];

        let shorthand = self.operation_defs.is_empty()
            && operation_type == &OperationType::Query
            && self.u.arbitrary().unwrap_or(false);

        Ok(OperationDef {
            operation_type: *operation_type,
            name,
            variable_definitions,
            directives,
            selection_set,
            shorthand,
        })
    }
}
