use std::collections::{HashMap, HashSet, VecDeque};

use itertools::Itertools;

use crate::{errors::SlipwayError, parse::types::primitives::ComponentHandle};

const CYCLE_DETECTED_ERROR: &str = "Cycle detected in the graph";

pub(crate) fn sort_and_group<'app>(
    components_and_dependencies: &HashMap<&'app ComponentHandle, HashSet<&'app ComponentHandle>>,
) -> Result<SortedAndGrouped<'app>, SlipwayError> {
    let graph = build_graph(components_and_dependencies);
    let sorted = graph.topological_sort()?;
    let grouped = graph.get_isolated_groups();
    Ok(SortedAndGrouped { sorted, grouped })
}

pub(crate) struct SortedAndGrouped<'app> {
    pub sorted: Vec<&'app ComponentHandle>,
    pub grouped: Vec<HashSet<&'app ComponentHandle>>,
}

fn build_graph<'app>(
    components: &HashMap<&'app ComponentHandle, HashSet<&'app ComponentHandle>>,
) -> Graph<'app> {
    let mut graph = Graph::new();

    for (&component_handle, input_handles) in components {
        graph.add_node(component_handle);
        for &input in input_handles {
            graph.add_edge(input, component_handle);
        }
    }

    graph
}

struct Graph<'a> {
    edges: HashMap<&'a ComponentHandle, HashSet<&'a ComponentHandle>>,
    in_degree: HashMap<&'a ComponentHandle, usize>,
}

impl<'a> Graph<'a> {
    fn new() -> Self {
        Graph {
            edges: HashMap::new(),
            in_degree: HashMap::new(),
        }
    }

    fn add_edge(&mut self, source: &'a ComponentHandle, destination: &'a ComponentHandle) {
        self.edges.entry(source).or_default().insert(destination);
        *self.in_degree.entry(destination).or_insert(0) += 1;
        self.in_degree.entry(source).or_insert(0);
    }

    fn add_node(&mut self, node: &'a ComponentHandle) {
        self.edges.entry(node).or_default();
        self.in_degree.entry(node).or_insert(0);
    }

    fn find_cycle(&self) -> Option<Vec<&'a ComponentHandle>> {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        // We sort before iterating to create a predictable sorted order.
        for &node in self.edges.keys().sorted() {
            if !stack.is_empty() {
                panic!("Stack should be empty at the start of each find_cycle_inner call");
            }

            if let Some(path) = self.find_cycle_inner(node, &mut visited, &mut stack) {
                return Some(path);
            }
        }

        None
    }

    fn find_cycle_inner(
        &self,
        node: &'a ComponentHandle,
        visited: &mut HashSet<&'a ComponentHandle>,
        stack: &mut Vec<&'a ComponentHandle>,
    ) -> Option<Vec<&'a ComponentHandle>> {
        if stack.contains(&node) {
            let mut path = vec![node];
            while let Some(&n) = stack.last() {
                stack.pop();
                path.push(n);
                if n == node {
                    path.reverse();
                    return Some(path);
                }
            }
        }

        if visited.insert(node) {
            stack.push(node);

            if let Some(neighbors) = self.edges.get(&node) {
                for &neighbor in neighbors.iter().sorted() {
                    if let Some(path) = self.find_cycle_inner(neighbor, visited, stack) {
                        return Some(path);
                    }
                }
            }

            stack.pop();
        }

        None
    }

    fn detect_cycle(&self) -> Result<(), SlipwayError> {
        if let Some(path) = self.find_cycle() {
            let cycle = path
                .iter()
                .map(|&x| x.to_string())
                .collect::<Vec<String>>()
                .join(" -> ");
            return Err(SlipwayError::ValidationFailed(format!(
                "{}: {}",
                CYCLE_DETECTED_ERROR, cycle
            )));
        }

        Ok(())
    }

    fn topological_sort(&self) -> Result<Vec<&'a ComponentHandle>, SlipwayError> {
        self.detect_cycle()?;

        let mut in_degree = self.in_degree.clone();
        let mut queue = VecDeque::new();

        // We sort before iterating to create a predictable sorted order.
        for &node in in_degree.keys().sorted() {
            let &degree = in_degree
                .get(node)
                .expect("Node should have an in_degree entry");
            if degree == 0 {
                queue.push_back(node);
            }
        }

        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node);

            if let Some(neighbors) = self.edges.get(&node) {
                // We sort before iterating to create a predictable sorted order.
                for &neighbor in neighbors.iter().sorted() {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        if order.len() != self.in_degree.len() {
            panic!("Graph appears to have a cycle which was not detected by detect_cycle()");
        }

        Ok(order)
    }

    // "Weakly connected components" are isolated groups of nodes in the graph.
    // https://en.wikipedia.org/wiki/Weak_component
    fn get_isolated_groups(&self) -> Vec<HashSet<&'a ComponentHandle>> {
        let mut visited: HashSet<&ComponentHandle> = HashSet::new();
        let mut result: Vec<HashSet<&ComponentHandle>> = Vec::new();

        let bidirectional_edges = self.get_bidirectional_edges();

        for &node in bidirectional_edges.keys() {
            if visited.contains(node) {
                continue;
            }

            let mut group: HashSet<&ComponentHandle> = HashSet::new();
            let mut queue: VecDeque<&ComponentHandle> = VecDeque::new();

            queue.push_back(node);

            while let Some(current) = queue.pop_front() {
                visited.insert(current);
                group.insert(current);

                for &neighbor in bidirectional_edges.get(current).unwrap_or(&HashSet::new()) {
                    if !visited.contains(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }

            result.push(group);
        }

        result
    }

    fn get_bidirectional_edges(
        &self,
    ) -> HashMap<&'a ComponentHandle, HashSet<&'a ComponentHandle>> {
        let mut bidirectional_edges = self.edges.clone();

        for (&node, connections) in self.edges.iter() {
            for &connected_node in connections {
                bidirectional_edges
                    .entry(connected_node)
                    .or_default()
                    .insert(node);
            }
        }

        bidirectional_edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_component_references(
        components: &HashMap<ComponentHandle, HashSet<ComponentHandle>>,
    ) -> HashMap<&ComponentHandle, HashSet<&ComponentHandle>> {
        let component_references: HashMap<&ComponentHandle, HashSet<&ComponentHandle>> = components
            .iter()
            .map(|(k, v)| (k, v.iter().collect()))
            .collect();
        component_references
    }

    mod topological_sort {
        use crate::test_utils::{ch, ch_set};

        use super::*;

        fn topological_sort<'app>(
            components_and_dependencies: &HashMap<
                &'app ComponentHandle,
                HashSet<&'app ComponentHandle>,
            >,
        ) -> Result<Vec<&'app ComponentHandle>, SlipwayError> {
            let graph = build_graph(components_and_dependencies);
            graph.topological_sort()
        }

        fn create_test_components() -> HashMap<ComponentHandle, HashSet<ComponentHandle>> {
            // Dependency graph:
            // C
            // |\
            // B |
            // \ /
            //  A
            vec![
                (ch("A"), ch_set(vec!["B", "C"])),
                (ch("B"), ch_set(vec!["C"])),
                (ch("C"), ch_set(vec![])),
            ]
            .into_iter()
            .collect()
        }

        #[test]
        fn test_graph_construction() {
            let components = create_test_components();
            let components = get_component_references(&components);

            let graph = build_graph(&components);

            assert_eq!(graph.edges.len(), 3);
            assert_eq!(graph.in_degree.len(), 3);
            assert_eq!(graph.in_degree.get(&ch("A")), Some(&2));
            assert_eq!(graph.in_degree.get(&ch("B")), Some(&1));
            assert_eq!(graph.in_degree.get(&ch("C")), Some(&0));
        }

        #[test]
        fn test_topological_sort_no_cycle() {
            let components = create_test_components();
            let components = get_component_references(&components);
            let order = topological_sort(&components).unwrap();
            assert_eq!(order, vec![&ch("C"), &ch("B"), &ch("A")]);
        }

        #[test]
        fn test_topological_sort_with_cycle() {
            // Dependency graph:
            // C--\
            // |\  |
            // B | ^
            // \ / |
            //  A-/

            let mut components = create_test_components();
            components.remove(&ch("C"));
            components.insert(ch("C"), vec![ch("A")].into_iter().collect());
            let components = get_component_references(&components);

            let result = topological_sort(&components);

            assert!(result.is_err());

            match result {
                Err(SlipwayError::ValidationFailed(msg)) => {
                    // There are a few cycles it could report, e.g. C -> B -> A -> C, but
                    // sorting during cycle detection ensures it always reports the same one.
                    assert_eq!(msg, format!("{}: {}", CYCLE_DETECTED_ERROR, "A -> C -> A"));
                }
                _ => panic!("Expected a ValidationFailed error"),
            }
        }

        #[test]
        fn test_graph_with_isolated_node() {
            let mut components = create_test_components();
            components.insert(ch("D"), HashSet::new());

            let components = get_component_references(&components);
            let graph = build_graph(&components);
            let order = graph.topological_sort().unwrap();

            assert!(order.contains(&&ch("D")));
        }

        #[test]
        fn test_topological_sort_large_graph() {
            // Dependency graph:
            //     C
            //    /|\
            //   F B \
            //  / / \|
            // | E   A
            // | |   |
            // | |   D
            // | \  /
            //  \  G  I J
            //   \ | / /
            //     H -/  K

            let mut components = create_test_components();

            // Add more components with dependencies
            components.insert(ch("D"), ch_set(vec!["A"]));
            components.insert(ch("E"), ch_set(vec!["B"]));
            components.insert(ch("F"), ch_set(vec!["C"]));
            components.insert(ch("G"), ch_set(vec!["D", "E"]));
            components.insert(
                ch("H"),
                ch_set(vec![
                    "F", "G",
                    // Note: This is the only mention of I in the graph. It implicitly has no input components.
                    "I", "J",
                ]),
            );
            components.insert(ch("J"), HashSet::new());
            components.insert(ch("K"), HashSet::new());

            let components = get_component_references(&components);
            let order = topological_sort(&components).unwrap();

            fn assert_order(order: &[&ComponentHandle], a: &str, b: &str) {
                assert!(
                    order.iter().position(|&x| x == &ch(a)).unwrap()
                        < order.iter().position(|&x| x == &ch(b)).unwrap(),
                    "Expected {} to be before {}",
                    a,
                    b
                );
            }

            // Check that all the inputs are before their components in the order.
            for (component, inputs) in components.iter() {
                assert!(order.contains(component));

                for input in inputs {
                    assert!(order.contains(input));

                    assert_order(&order, &input.to_string(), &component.0.to_string());
                }
            }

            // Our topological sort should have a consistent order due to sorting.
            assert_eq!(
                order.iter().join(" -> "),
                "C -> I -> J -> K -> B -> F -> A -> E -> D -> G -> H"
            );
        }
    }

    mod get_isolated_groups {
        use crate::test_utils::{ch, ch_set};

        use super::*;

        #[test]
        fn test_empty_graph() {
            let graph = Graph {
                edges: HashMap::new(),
                in_degree: HashMap::new(),
            };
            let components = graph.get_isolated_groups();
            assert!(
                components.is_empty(),
                "Expected no components for an empty graph"
            );
        }

        #[test]
        fn test_single_node_graph() {
            let mut edges = HashMap::new();
            let mut in_degree = HashMap::new();
            let node = ch("A");
            edges.insert(&node, HashSet::new());
            in_degree.insert(&node, 0);

            let graph = Graph { edges, in_degree };
            let components = graph.get_isolated_groups();
            assert_eq!(
                components.len(),
                1,
                "Expected a single component for a single-node graph"
            );
            assert_eq!(
                components[0].len(),
                1,
                "Expected single component to contain a single node"
            );
            assert!(
                components[0].contains(&node),
                "Component should contain the single node"
            );
        }

        #[test]
        fn test_single_weakly_connected_component() {
            // Dependency graph:
            // A
            // |
            // B
            // |
            // C
            let components = vec![
                (ch("A"), ch_set(vec![])),
                (ch("B"), ch_set(vec!["A"])),
                (ch("C"), ch_set(vec!["B"])),
            ]
            .into_iter()
            .collect();

            let components = get_component_references(&components);
            let graph = build_graph(&components);

            let components = graph.get_isolated_groups();
            assert_eq!(
                components.len(),
                1,
                "Expected all nodes to be in a single component"
            );
        }

        #[test]
        fn test_multiple_weakly_connected_components() {
            // Dependency graph:
            // A
            // |
            // B
            // |
            // C
            //
            // D
            //
            // E
            // |\
            // F G
            let components = vec![
                (ch("A"), ch_set(vec![])),
                (ch("B"), ch_set(vec!["A"])),
                (ch("C"), ch_set(vec!["B"])),
                (ch("D"), ch_set(vec![])),
                (ch("E"), ch_set(vec![])),
                (ch("F"), ch_set(vec!["E"])),
                (ch("G"), ch_set(vec!["E"])),
            ]
            .into_iter()
            .collect();

            let components = get_component_references(&components);
            let graph = build_graph(&components);

            let components = graph.get_isolated_groups();
            assert_eq!(
                components.len(),
                3,
                "Expected all nodes to be in a single component"
            );
        }

        #[test]
        fn test_cycles_in_graph() {
            // Dependency graph:
            // A
            // |\
            // B |
            // |/
            // C
            //
            // D
            let components = vec![
                (ch("A"), ch_set(vec!["C"])),
                (ch("B"), ch_set(vec!["A"])),
                (ch("C"), ch_set(vec!["B"])),
                (ch("D"), ch_set(vec![])),
            ]
            .into_iter()
            .collect();

            let components = get_component_references(&components);
            let graph = build_graph(&components);

            let components = graph.get_isolated_groups();
            assert_eq!(
                components.len(),
                2,
                "Expected all nodes to be in a single component"
            );
        }
    }
}
