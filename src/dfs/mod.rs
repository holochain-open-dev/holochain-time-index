use hdk::hash_path::path::{Component, Path};
use hdk::prelude::*;
use petgraph::{dot::Dot, graph::NodeIndex, stable_graph::StableDiGraph};
use std::convert::TryFrom;

use crate::entries::{Index, IndexIndex, TimeIndex};
use crate::errors::IndexError;

pub(crate) mod methods;

#[derive(Debug)]
pub(crate) struct SearchState(pub StableDiGraph<GraphTimeItem, ()>);

pub(crate) struct GraphTimeItem(pub Vec<Component>);

impl std::fmt::Debug for GraphTimeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut components: Vec<Component> = self.0.clone().into();
        let mut debug_struct = f.debug_struct("Path");
        if components.len() > 0 {
            debug_struct.field(
                "index",
                &IndexIndex::try_from(components[0].clone()).unwrap().0,
            );
            components.remove(0);
        };
        for component in components {
            let time_index = TimeIndex::try_from(component.clone());
            if time_index.is_err() {
                debug_struct.field(
                    "index",
                    &Index::try_from(component)
                        .expect("Could not convert component into TimeIndex or IndexIndex"),
                )
            } else {
                debug_struct.field(
                    "time_index",
                    &time_index
                        .expect("Could not convert component into TimeIndex or IndexIndex")
                        .0,
                )
            };
        }
        debug_struct.finish()
    }
}

impl SearchState {
    /// Create a new petgraph::StableDiGraph
    pub(crate) fn new() -> SearchState {
        SearchState(StableDiGraph::new())
    }

    /// Given an *empty* graph populate it with paths
    pub(crate) fn populate_from_paths(
        &mut self,
        paths: &Vec<Path>,
        depth: usize,
    ) -> Result<(), IndexError> {
        for (path_i, path) in paths.iter().enumerate() {
            let components: Vec<Component> = path.clone().into();
            for (i, _component) in components.iter().enumerate() {
                if i != components.len() && i >= depth {
                    let i1 = self
                        .0
                        .add_node(GraphTimeItem(components[0..i + 1].to_owned()));
                    //self.0.add_edge(i1, i2, ());
                    if i != 0 {
                        if depth == 0 {
                            self.0.add_edge(NodeIndex::new(i1.index() - 1), i1, ());
                        } else {
                            self.0
                                .add_edge(NodeIndex::new(i1.index() - (path_i + 1)), i1, ());
                        }
                    };
                }
            }
        }
        Ok(())
    }

    /// Given a graph and node position add paths onto node where paths only get added if they contain more components than value of depth
    pub(crate) fn populate_from_paths_forward(
        &mut self,
        paths: Vec<Path>,
        depth: usize,
        parent_node: NodeIndex,
    ) -> Result<NodeIndex, IndexError> {
        let mut first_node_index = NodeIndex::new(0);
        for (path_i, path) in paths.iter().enumerate() {
            let components: Vec<Component> = path.clone().into();
            for (i, _component) in components.iter().enumerate() {
                if i != components.len() && i >= depth {
                    //Only insert into the graph the paths where the number of components is == depth we are serving
                    let i1 = self
                        .0
                        .add_node(GraphTimeItem(components[0..i + 1].to_owned()));
                    //If this is the first element in the paths, save and return so future paths can link from here
                    if path_i == 0 {
                        first_node_index = i1;
                    };
                    self.0.add_edge(parent_node, i1, ());
                }
            }
        }
        Ok(first_node_index)
    }

    /// Given paths add them to graph and add edge from given position pointing to each newly added path
    pub(crate) fn populate_next_nodes_from_position(
        &mut self,
        paths: Vec<Path>,
        position: NodeIndex,
    ) -> Result<Vec<NodeIndex>, IndexError> {
        let mut added_indexes = vec![];
        for path in paths {
            let components: Vec<Component> = path.clone().into();
            let i1 = self.0.add_node(GraphTimeItem(components));
            added_indexes.push(i1);
            self.0.add_edge(position, i1, ());
        }
        Ok(added_indexes)
    }

    /// Holochain debug dot representation of graph state
    pub(crate) fn display_dot_repr(&self) {
        debug!("{:#?}", Dot::new(&self.0));
    }
}
