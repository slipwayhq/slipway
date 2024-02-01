use std::collections::{HashMap, HashSet, VecDeque};

use itertools::Itertools;

use crate::{errors::SlipwayError, rigging::parse::ComponentHandle};

use super::ComponentAndDependencies;

const CYCLE_DETECTED_ERROR: &str = "Cycle detected in the graph";

pub(crate) fn topological_sort(
    components_and_dependencies: &[ComponentAndDependencies],
) -> Result<Vec<&ComponentHandle>, SlipwayError> {
    let graph = build_graph(components_and_dependencies);
    graph.topological_sort()
}

fn build_graph(components: &[ComponentAndDependencies]) -> Graph {
    let mut graph = Graph::new();

    for component in components {
        graph.add_node(&component.component_handle);
        for input in &component.input_handles {
            graph.add_edge(input, &component.component_handle);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_components() -> Vec<ComponentAndDependencies> {
        // Dependency graph:
        // C
        // |\
        // B |
        // \ /
        //  A
        vec![
            ComponentAndDependencies {
                component_handle: ComponentHandle::for_test("A"),
                input_handles: vec![
                    ComponentHandle::for_test("B"),
                    ComponentHandle::for_test("C"),
                ]
                .into_iter()
                .collect(),
            },
            ComponentAndDependencies {
                component_handle: ComponentHandle::for_test("B"),
                input_handles: vec![ComponentHandle::for_test("C")].into_iter().collect(),
            },
            ComponentAndDependencies {
                component_handle: ComponentHandle::for_test("C"),
                input_handles: HashSet::new(),
            },
        ]
    }

    #[test]
    fn test_graph_construction() {
        let components = create_test_components();
        let graph = build_graph(&components);

        assert_eq!(graph.edges.len(), 3);
        assert_eq!(graph.in_degree.len(), 3);
        assert_eq!(
            graph.in_degree.get(&ComponentHandle::for_test("A")),
            Some(&2)
        );
        assert_eq!(
            graph.in_degree.get(&ComponentHandle::for_test("B")),
            Some(&1)
        );
        assert_eq!(
            graph.in_degree.get(&ComponentHandle::for_test("C")),
            Some(&0)
        );
    }

    #[test]
    fn test_topological_sort_no_cycle() {
        let components = create_test_components();
        let order = topological_sort(&components).unwrap();
        assert_eq!(
            order,
            vec![
                &ComponentHandle::for_test("C"),
                &ComponentHandle::for_test("B"),
                &ComponentHandle::for_test("A")
            ]
        );
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
        components.pop();
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("C"),
            input_handles: vec![ComponentHandle::for_test("A")].into_iter().collect(),
        });

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
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("D"),
            input_handles: HashSet::new(),
        });

        let graph = build_graph(&components);
        let order = graph.topological_sort().unwrap();

        assert!(order.contains(&&ComponentHandle::for_test("D")));
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
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("D"),
            input_handles: vec![ComponentHandle::for_test("A")].into_iter().collect(),
        });
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("E"),
            input_handles: vec![ComponentHandle::for_test("B")].into_iter().collect(),
        });
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("F"),
            input_handles: vec![ComponentHandle::for_test("C")].into_iter().collect(),
        });
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("G"),
            input_handles: vec![
                ComponentHandle::for_test("D"),
                ComponentHandle::for_test("E"),
            ]
            .into_iter()
            .collect(),
        });
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("H"),
            input_handles: vec![
                ComponentHandle::for_test("F"),
                ComponentHandle::for_test("G"),
                // Note: This is the only mention of I in the graph. It implicitly has no input components.
                ComponentHandle::for_test("I"),
                ComponentHandle::for_test("J"),
            ]
            .into_iter()
            .collect(),
        });
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("J"),
            input_handles: HashSet::new(),
        });
        components.push(ComponentAndDependencies {
            component_handle: ComponentHandle::for_test("K"),
            input_handles: HashSet::new(),
        });

        let order = topological_sort(&components).unwrap();

        fn assert_order(order: &[&ComponentHandle], a: &str, b: &str) {
            assert!(
                order
                    .iter()
                    .position(|&x| x == &ComponentHandle::for_test(a))
                    .unwrap()
                    < order
                        .iter()
                        .position(|&x| x == &ComponentHandle::for_test(b))
                        .unwrap(),
                "Expected {} to be before {}",
                a,
                b
            );
        }

        // Check that all the inputs are before their components in the order.
        for component in &components {
            assert!(order.contains(&&component.component_handle));

            for input in &component.input_handles {
                assert!(order.contains(&input));

                assert_order(
                    &order,
                    &input.to_string(),
                    &component.component_handle.to_string(),
                );
            }
        }

        // Our topological sort should have a consistent order due to sorting.
        assert_eq!(
            order.iter().join(" -> "),
            "C -> I -> J -> K -> B -> F -> A -> E -> D -> G -> H"
        );
    }
}
