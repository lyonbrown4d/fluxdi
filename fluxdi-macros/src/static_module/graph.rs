use std::collections::{HashMap, HashSet};

use super::{Argument, Module, Scope, key};

pub(super) struct Graph {
    pub(super) order: Vec<usize>,
    pub(super) dependencies: Vec<Vec<usize>>,
    pub(super) is_async: Vec<bool>,
    pub(super) arguments: Vec<Vec<std::ops::Range<usize>>>,
    edge_arguments: Vec<Vec<usize>>,
}

impl Module {
    pub(super) fn validate(&self) -> syn::Result<Graph> {
        let mut names = HashMap::new();
        let mut errors: Option<syn::Error> = None;
        let export_names = self
            .bindings
            .iter()
            .flat_map(|binding| {
                ["provider", "root_safe", "is_async"]
                    .map(|kind| format!("__fluxdi_{kind}_{}", key(&binding.name)))
            })
            .collect::<HashSet<_>>();
        let mut aliases = HashSet::new();
        for import in &self.imports {
            let name = key(&import.name);
            if !aliases.insert(name.clone()) {
                combine(
                    &mut errors,
                    syn::Error::new_spanned(
                        &import.name,
                        format!("duplicate static import alias `{name}`"),
                    ),
                );
            }
        }
        for (index, binding) in self.bindings.iter().enumerate() {
            let name = key(&binding.name);
            if matches!(
                name.as_str(),
                "new"
                    | "create_scope"
                    | "create_child"
                    | "injector"
                    | "in_context"
                    | "clone"
                    | "default"
            ) || export_names.contains(&name)
            {
                combine(
                    &mut errors,
                    syn::Error::new_spanned(
                        &binding.name,
                        format!("binding `{name}` conflicts with a generated module method"),
                    ),
                );
            }
            if names.insert(name.clone(), index).is_some() {
                combine(
                    &mut errors,
                    syn::Error::new_spanned(
                        &binding.name,
                        format!("duplicate static binding `{name}`"),
                    ),
                );
            }
        }
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, binding) in self.bindings.iter().enumerate() {
            for group in &binding.groups {
                groups.entry(key(group)).or_default().push(index);
            }
        }
        let mut dependencies = vec![Vec::new(); self.bindings.len()];
        let mut arguments = vec![Vec::new(); self.bindings.len()];
        let mut edge_arguments = vec![Vec::new(); self.bindings.len()];
        for (index, binding) in self.bindings.iter().enumerate() {
            if let Some(factory) = &binding.factory {
                for (argument, dependency) in factory.dependencies.iter().enumerate() {
                    let start = dependencies[index].len();
                    match dependency {
                        Argument::All(group) => {
                            dependencies[index]
                                .extend(groups.get(&key(group)).into_iter().flatten());
                        }
                        Argument::Required(name) | Argument::Optional(name) => {
                            if let Some(target) = names.get(&key(name)) {
                                dependencies[index].push(*target);
                            } else if matches!(dependency, Argument::Required(_)) {
                                combine(
                                    &mut errors,
                                    syn::Error::new_spanned(
                                        name,
                                        format!(
                                            "unknown static binding `{}` required by `{}`",
                                            key(name),
                                            key(&binding.name)
                                        ),
                                    ),
                                );
                            }
                        }
                    }
                    let end = dependencies[index].len();
                    arguments[index].push(start..end);
                    edge_arguments[index].extend(std::iter::repeat_n(argument, end - start));
                }
            }
        }
        if let Some(errors) = errors {
            return Err(errors);
        }
        let mut graph = Graph {
            order: Vec::with_capacity(self.bindings.len()),
            dependencies,
            arguments,
            edge_arguments,
            is_async: self
                .bindings
                .iter()
                .map(|binding| binding.declared_async)
                .collect(),
        };
        let mut states = vec![0; self.bindings.len()];
        let mut chain = Vec::new();
        for index in 0..self.bindings.len() {
            self.visit(index, &mut graph, &mut states, &mut chain)?;
        }
        for &index in &graph.order {
            graph.is_async[index] |= graph.dependencies[index]
                .iter()
                .any(|target| graph.is_async[*target]);
        }
        for (index, binding) in self.bindings.iter().enumerate() {
            if binding.scope == Scope::Root && binding.imported.is_none() {
                let mut seen = vec![false; self.bindings.len()];
                self.check_root_capture(index, &graph, &mut seen, &mut vec![index])?;
            }
        }
        Ok(graph)
    }

    fn check_root_capture(
        &self,
        index: usize,
        graph: &Graph,
        seen: &mut [bool],
        chain: &mut Vec<usize>,
    ) -> syn::Result<()> {
        if seen[index] {
            return Ok(());
        }
        seen[index] = true;
        for (offset, dependency) in graph.dependencies[index].iter().copied().enumerate() {
            chain.push(dependency);
            if self.bindings[dependency].imported.is_none()
                && matches!(
                    self.bindings[dependency].scope,
                    Scope::Singleton | Scope::Scoped
                )
            {
                let path = chain
                    .iter()
                    .map(|item| key(&self.bindings[*item].name))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                let argument = graph.edge_arguments[index][offset];
                let edge =
                    self.bindings[index].factory.as_ref().unwrap().dependencies[argument].name();
                return Err(syn::Error::new_spanned(
                    edge,
                    format!(
                        "root binding `{}` cannot capture contextual binding `{}`: {path}",
                        key(&self.bindings[chain[0]].name),
                        key(&self.bindings[dependency].name),
                    ),
                ));
            }
            self.check_root_capture(dependency, graph, seen, chain)?;
            chain.pop();
        }
        Ok(())
    }

    fn visit(
        &self,
        index: usize,
        graph: &mut Graph,
        states: &mut [u8],
        chain: &mut Vec<usize>,
    ) -> syn::Result<()> {
        if states[index] == 2 {
            return Ok(());
        }
        states[index] = 1;
        chain.push(index);
        for offset in 0..graph.dependencies[index].len() {
            let dependency = graph.dependencies[index][offset];
            if states[dependency] == 1 {
                let start = chain.iter().position(|item| *item == dependency).unwrap();
                let cycle = chain[start..]
                    .iter()
                    .chain(std::iter::once(&dependency))
                    .map(|item| key(&self.bindings[*item].name))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                let argument = graph.edge_arguments[index][offset];
                let edge =
                    self.bindings[index].factory.as_ref().unwrap().dependencies[argument].name();
                return Err(syn::Error::new_spanned(
                    edge,
                    format!("static dependency cycle: {cycle}"),
                ));
            }
            if states[dependency] == 0 {
                self.visit(dependency, graph, states, chain)?;
            }
        }
        chain.pop();
        states[index] = 2;
        graph.order.push(index);
        Ok(())
    }
}

fn combine(errors: &mut Option<syn::Error>, error: syn::Error) {
    if let Some(errors) = errors {
        errors.combine(error);
    } else {
        *errors = Some(error);
    }
}
