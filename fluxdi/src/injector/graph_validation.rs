use super::*;

impl Injector {
    fn build_dependency_graph_and_report(&self) -> (DependencyGraph, GraphValidationReport) {
        let mut state = GraphBuildState::default();
        self.collect_graph_state(&mut state);

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut report = GraphValidationReport::default();

        let mut single_ids: HashMap<TypeId, String> = HashMap::new();
        let mut named_ids: HashMap<NamedTypeKey, String> = HashMap::new();
        let mut set_ids: HashMap<TypeId, Vec<String>> = HashMap::new();
        let mut meta_by_node: HashMap<String, ProviderGraphMeta> = HashMap::new();

        let mut singles: Vec<ProviderGraphMeta> = state.singles.into_values().collect();
        singles.sort_by(|a, b| a.type_name.cmp(b.type_name));

        for meta in singles {
            let node_id = format!("single::{}", meta.type_name);
            single_ids.insert(meta.type_id, node_id.clone());
            meta_by_node.insert(node_id.clone(), meta.clone());

            nodes.push(GraphNode {
                id: node_id,
                type_name: meta.type_name.to_string(),
                scope: meta.scope,
                binding: GraphBinding::Single,
                dependencies: meta
                    .dependencies
                    .iter()
                    .map(|dep| GraphDependency {
                        type_name: dep.type_name.to_string(),
                        name: dep.name.clone(),
                        cardinality: dep.cardinality,
                    })
                    .collect(),
            });
        }

        let mut named: Vec<(NamedTypeKey, ProviderGraphMeta)> = state.named.into_iter().collect();
        named.sort_by(|(a_key, a_meta), (b_key, b_meta)| {
            (a_meta.type_name, a_key.name.as_str()).cmp(&(b_meta.type_name, b_key.name.as_str()))
        });

        for (key, meta) in named {
            let node_id = format!("named::{}::{}", meta.type_name, key.name);
            named_ids.insert(key.clone(), node_id.clone());
            meta_by_node.insert(node_id.clone(), meta.clone());

            nodes.push(GraphNode {
                id: node_id,
                type_name: meta.type_name.to_string(),
                scope: meta.scope,
                binding: GraphBinding::Named(key.name.clone()),
                dependencies: meta
                    .dependencies
                    .iter()
                    .map(|dep| GraphDependency {
                        type_name: dep.type_name.to_string(),
                        name: dep.name.clone(),
                        cardinality: dep.cardinality,
                    })
                    .collect(),
            });
        }

        let mut sets: Vec<(TypeId, Vec<ProviderGraphMeta>)> = state.sets.into_iter().collect();
        sets.sort_by(|(_, a), (_, b)| {
            let left = a.first().map(|meta| meta.type_name).unwrap_or("");
            let right = b.first().map(|meta| meta.type_name).unwrap_or("");
            left.cmp(right)
        });

        for (type_id, metas) in sets {
            let set_type_name = metas
                .first()
                .map(|meta| meta.type_name)
                .unwrap_or("unknown");

            for (index, meta) in metas.into_iter().enumerate() {
                let node_id = format!("set::{}::{}", set_type_name, index);
                set_ids.entry(type_id).or_default().push(node_id.clone());
                meta_by_node.insert(node_id.clone(), meta.clone());

                nodes.push(GraphNode {
                    id: node_id,
                    type_name: meta.type_name.to_string(),
                    scope: meta.scope,
                    binding: GraphBinding::Set(index),
                    dependencies: meta
                        .dependencies
                        .iter()
                        .map(|dep| GraphDependency {
                            type_name: dep.type_name.to_string(),
                            name: dep.name.clone(),
                            cardinality: dep.cardinality,
                        })
                        .collect(),
                });
            }
        }

        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for node in &nodes {
            let meta = match meta_by_node.get(&node.id) {
                Some(meta) => meta,
                None => continue,
            };

            for dep in &meta.dependencies {
                let targets: Vec<String> = match dep.cardinality {
                    DependencyCardinality::One | DependencyCardinality::Optional => {
                        if let Some(name) = dep.name.clone() {
                            named_ids
                                .get(&NamedTypeKey {
                                    type_id: dep.type_id,
                                    name,
                                })
                                .cloned()
                                .into_iter()
                                .collect()
                        } else {
                            single_ids.get(&dep.type_id).cloned().into_iter().collect()
                        }
                    }
                    DependencyCardinality::All => {
                        set_ids.get(&dep.type_id).cloned().unwrap_or_default()
                    }
                };

                if targets.is_empty() {
                    if dep.cardinality == DependencyCardinality::Optional {
                        continue;
                    }
                    let message = match dep.cardinality {
                        DependencyCardinality::One | DependencyCardinality::Optional => {
                            if let Some(name) = &dep.name {
                                format!(
                                    "Graph validation failed: node {} depends on missing named dependency {} ({})",
                                    node.id, dep.type_name, name
                                )
                            } else {
                                format!(
                                    "Graph validation failed: node {} depends on missing dependency {}",
                                    node.id, dep.type_name
                                )
                            }
                        }
                        DependencyCardinality::All => format!(
                            "Graph validation failed: node {} depends on missing set dependency {}",
                            node.id, dep.type_name
                        ),
                    };

                    report.issues.push(GraphValidationIssue {
                        kind: GraphValidationIssueKind::MissingDependency,
                        node_id: Some(node.id.clone()),
                        message,
                    });
                    continue;
                }

                for target in targets {
                    adjacency
                        .entry(node.id.clone())
                        .or_default()
                        .push(target.clone());

                    edges.push(GraphEdge {
                        from: node.id.clone(),
                        to: target,
                        label: dependency_label(dep.cardinality, dep.name.as_deref()),
                    });
                }
            }
        }

        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut seen_signatures = HashSet::new();

        let mut sorted_nodes: Vec<String> = nodes.iter().map(|node| node.id.clone()).collect();
        sorted_nodes.sort();

        for node_id in sorted_nodes {
            detect_cycles(
                node_id.as_str(),
                &adjacency,
                &mut visiting,
                &mut visited,
                &mut stack,
                &mut seen_signatures,
                &mut report,
            );
        }

        edges.sort_by(|left, right| {
            (
                left.from.as_str(),
                left.to.as_str(),
                left.label.as_deref().unwrap_or(""),
            )
                .cmp(&(
                    right.from.as_str(),
                    right.to.as_str(),
                    right.label.as_deref().unwrap_or(""),
                ))
        });

        (DependencyGraph { nodes, edges }, report)
    }

    pub fn dependency_graph(&self) -> DependencyGraph {
        self.build_dependency_graph_and_report().0
    }

    pub fn validate_graph(&self) -> GraphValidationReport {
        self.build_dependency_graph_and_report().1
    }

    pub fn try_validate_graph(&self) -> Result<(), Error> {
        let report = self.validate_graph();
        if report.is_valid() {
            return Ok(());
        }

        Err(Error::graph_validation_failed(report.summary().as_str()))
    }
}
