//! Which cell reads which, and the order to calculate them in.
//!
//! The two graph algorithms the calculation order needs: a topological order
//! and the strongly connected components. They were petgraph's (`toposort`
//! and `tarjan_scc`), which pulled a second hashbrown into the tree
//! (ADR-004: every crate in one version). These are the same algorithms,
//! visiting nodes and edges in the same order, so the calculation order, and
//! which error is reported first, is the one petgraph gave: a test checks the
//! two agree on thousands of graphs.

/// A directed graph. Nodes are numbered in the order they are added; the
/// successors of a node are visited last added first, as in petgraph.
pub(super) struct Graph<N> {
    nodes: Vec<N>,
    edges: Vec<(usize, usize)>,
    /// The successors of each node, in the order their edges were added
    successors: Vec<Vec<usize>>,
}

impl<N> Default for Graph<N> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            successors: Vec::new(),
        }
    }
}

impl<N> std::ops::Index<usize> for Graph<N> {
    type Output = N;

    fn index(&self, node: usize) -> &N {
        &self.nodes[node]
    }
}

impl<N> Graph<N> {
    /// Add a node and return its number.
    pub(super) fn add_node(&mut self, weight: N) -> usize {
        self.nodes.push(weight);
        self.successors.push(Vec::new());
        self.nodes.len() - 1
    }

    /// Add an edge, even if the same one is already there.
    pub(super) fn add_edge(&mut self, from: usize, to: usize) {
        self.edges.push((from, to));
        self.successors[from].push(to);
    }

    /// Add an edge unless it is already there.
    pub(super) fn update_edge(&mut self, from: usize, to: usize) {
        if !self.contains_edge(from, to) {
            self.add_edge(from, to);
        }
    }

    /// Whether there is an edge from `from` to `to`.
    pub(super) fn contains_edge(&self, from: usize, to: usize) -> bool {
        self.successors[from].contains(&to)
    }

    /// The edges, in the order they were added.
    pub(super) fn edges(&self) -> &[(usize, usize)] {
        &self.edges
    }

    /// The `nth` successor of `node`, the last added first.
    fn successor(&self, node: usize, nth: usize) -> Option<usize> {
        self.successors[node].iter().rev().nth(nth).copied()
    }

    /// The nodes in an order in which every edge goes forward, or `None` if
    /// there is a cycle.
    ///
    /// Depth first from each node in turn, the order being the reverse of
    /// the order in which nodes are finished (petgraph's `toposort`).
    pub(super) fn topological_order(&self) -> Option<Vec<usize>> {
        let count = self.nodes.len();
        let mut discovered = vec![false; count];
        let mut finished = vec![false; count];
        let mut order = Vec::with_capacity(count);
        let mut stack = Vec::new();
        for start in 0..count {
            if discovered[start] {
                continue;
            }
            stack.push(start);
            while let Some(&node) = stack.last() {
                if discovered[node] {
                    stack.pop();
                    if !finished[node] {
                        finished[node] = true;
                        order.push(node);
                    }
                } else {
                    // First time on top: its successors go on the stack, it stays
                    discovered[node] = true;
                    stack.extend(
                        self.successors[node]
                            .iter()
                            .rev()
                            .filter(|&&next| !discovered[next]),
                    );
                }
            }
        }
        order.reverse();
        // Without a cycle every edge goes forward in that order; with one, no
        // order has every edge forward
        let mut position = vec![0; count];
        for (at, &node) in order.iter().enumerate() {
            position[node] = at;
        }
        self.edges
            .iter()
            .all(|&(from, to)| position[from] < position[to])
            .then_some(order)
    }

    /// The strongly connected components: groups of nodes that each reach
    /// all the others, and each node on its own that is in no such group.
    /// A component comes after every component it reaches.
    ///
    /// Pearce's version of Tarjan's algorithm, as petgraph's `tarjan_scc`,
    /// with a stack of its own instead of recursion, so a long chain of
    /// formulas cannot overflow the thread's stack.
    pub(super) fn strongly_connected(&self) -> Vec<Vec<usize>> {
        // A node's root index: 0 until visited, then the smallest index it
        // reaches, then the component's number once it has one
        let mut root = vec![0; self.nodes.len()];
        let mut index = 1;
        let mut component_number = usize::MAX;
        let mut stack: Vec<usize> = Vec::new();
        let mut components = Vec::new();
        // The nodes being visited: the node, how many of its successors are
        // looked at, and whether it is the root of its component so far
        let mut visiting: Vec<(usize, usize, bool)> = Vec::new();
        for start in 0..self.nodes.len() {
            if root[start] != 0 {
                continue;
            }
            root[start] = index;
            index += 1;
            visiting.push((start, 0, true));
            while let Some(&(node, looked_at, is_root)) = visiting.last() {
                if let Some(next) = self.successor(node, looked_at) {
                    let top = visiting.len() - 1;
                    visiting[top].1 += 1;
                    if root[next] == 0 {
                        root[next] = index;
                        index += 1;
                        visiting.push((next, 0, true));
                    } else {
                        lower_root(&mut root, &mut visiting, next);
                    }
                    continue;
                }
                visiting.pop();
                if is_root {
                    // The node and those above it on the stack are a component
                    let mut adjustment = 1;
                    let own = root[node];
                    let first = stack
                        .iter()
                        .rposition(|&other| {
                            if own > root[other] {
                                true
                            } else {
                                root[other] = component_number;
                                adjustment += 1;
                                false
                            }
                        })
                        .map_or(0, |at| at + 1);
                    root[node] = component_number;
                    stack.push(node);
                    components.push(stack.split_off(first));
                    index -= adjustment;
                    component_number -= 1;
                } else {
                    stack.push(node);
                }
                lower_root(&mut root, &mut visiting, node);
            }
        }
        components
    }
}

/// After looking at `next`, a successor of the node being visited: if `next`
/// has the smaller root index, the node takes it, and is not the root of its
/// component.
const fn lower_root(root: &mut [usize], visiting: &mut [(usize, usize, bool)], next: usize) {
    if let Some((node, _, is_root)) = visiting.last_mut() {
        if root[next] < root[*node] {
            root[*node] = root[next];
            *is_root = false;
        }
    }
}

#[cfg(test)]
mod tests {
    //! The order and the components, worked out by hand on small graphs, and
    //! compared with petgraph's on many random ones: the calculation order
    //! must not change with the algorithm.

    use super::Graph;
    use rand::rngs::StdRng;
    use rand::{RngExt, SeedableRng};

    /// A graph of `count` nodes, numbered as their weights, and its edges
    fn graph(count: usize, edges: &[(usize, usize)]) -> Graph<usize> {
        let mut graph = Graph::default();
        for node in 0..count {
            assert_eq!(graph.add_node(node), node);
        }
        for &(from, to) in edges {
            graph.add_edge(from, to);
        }
        graph
    }

    #[test]
    fn test_a_chain_is_ordered_along_its_edges() {
        let chain = graph(3, &[(2, 1), (1, 0)]);
        assert_eq!(chain.topological_order(), Some(vec![2, 1, 0]));
        assert_eq!(chain[1], 1);
        assert_eq!(chain.edges(), &[(2, 1), (1, 0)]);
    }

    #[test]
    fn test_unconnected_nodes_keep_the_order_of_the_last_added_first() {
        // Each node is finished as it is reached, and the order is reversed
        assert_eq!(graph(3, &[]).topological_order(), Some(vec![2, 1, 0]));
        assert_eq!(Graph::<usize>::default().topological_order(), Some(vec![]));
    }

    #[test]
    fn test_a_diamond_visits_the_last_added_successor_first() {
        // 0 reads into 1 and 2, both into 3. The successors of 0 go on the
        // stack 2 then 1, so 1 is finished first, then 2
        let diamond = graph(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
        assert_eq!(diamond.topological_order(), Some(vec![0, 2, 1, 3]));
    }

    #[test]
    fn test_a_cycle_has_no_order() {
        assert_eq!(graph(2, &[(0, 1), (1, 0)]).topological_order(), None);
        assert_eq!(
            graph(3, &[(0, 1), (1, 2), (2, 0)]).topological_order(),
            None
        );
        assert_eq!(graph(1, &[(0, 0)]).topological_order(), None);
    }

    #[test]
    fn test_components_come_after_those_they_reach() {
        // 0 and 1 read each other and are read by 2; 3 reads itself
        let components = graph(4, &[(0, 1), (1, 0), (2, 0), (3, 3)]).strongly_connected();
        assert_eq!(components, vec![vec![1, 0], vec![2], vec![3]]);
    }

    #[test]
    fn test_a_long_chain_does_not_overflow_the_stack() {
        let count = 200_000;
        let edges: Vec<(usize, usize)> = (1..count).map(|node| (node - 1, node)).collect();
        let chain = graph(count, &edges);
        let components = chain.strongly_connected();
        assert_eq!(components.len(), count);
        assert_eq!(components[0], vec![count - 1]);
        assert_eq!(chain.topological_order().map(|order| order[0]), Some(0));
    }

    #[test]
    fn test_update_edge_adds_an_edge_once() {
        let mut two = graph(2, &[]);
        two.update_edge(0, 1);
        two.update_edge(0, 1);
        assert_eq!(two.edges(), &[(0, 1)]);
        assert!(two.contains_edge(0, 1));
        assert!(!two.contains_edge(1, 0));
    }

    /// The same graph in petgraph
    fn petgraph_of(count: usize, edges: &[(usize, usize)]) -> petgraph::graph::DiGraph<usize, ()> {
        let mut graph = petgraph::graph::DiGraph::new();
        for node in 0..count {
            graph.add_node(node);
        }
        for &(from, to) in edges {
            graph.add_edge(
                petgraph::graph::NodeIndex::new(from),
                petgraph::graph::NodeIndex::new(to),
                (),
            );
        }
        graph
    }

    #[test]
    fn test_the_same_order_and_components_as_petgraph() {
        let mut rng = StdRng::seed_from_u64(20_261_120);
        let (mut ordered, mut cyclic) = (0, 0);
        for _ in 0..5_000 {
            let count = rng.random_range(0..12_usize);
            let edge_count = if count == 0 {
                0
            } else {
                rng.random_range(0..count * 2)
            };
            // Mostly forward edges, so that many graphs have no cycle;
            // repeated edges and edges to the node itself as well
            let edges: Vec<(usize, usize)> = (0..edge_count)
                .map(|_| {
                    let (a, b) = (rng.random_range(0..count), rng.random_range(0..count));
                    if rng.random_range(0..8) == 0 || a == b {
                        (a, b)
                    } else {
                        (a.min(b), a.max(b))
                    }
                })
                .collect();
            let ours = graph(count, &edges);
            let theirs = petgraph_of(count, &edges);
            let expected = petgraph::algo::toposort(&theirs, None).ok().map(|order| {
                order
                    .into_iter()
                    .map(petgraph::graph::NodeIndex::index)
                    .collect()
            });
            let order = ours.topological_order();
            assert_eq!(order, expected, "order of {count} nodes, edges {edges:?}");
            if order.is_some() {
                ordered += 1;
            } else {
                cyclic += 1;
            }
            let expected: Vec<Vec<usize>> = petgraph::algo::tarjan_scc(&theirs)
                .into_iter()
                .map(|component| {
                    component
                        .into_iter()
                        .map(petgraph::graph::NodeIndex::index)
                        .collect()
                })
                .collect();
            assert_eq!(
                ours.strongly_connected(),
                expected,
                "components of {count} nodes, edges {edges:?}"
            );
        }
        // Both kinds of graph were compared, many of each
        assert!(ordered > 1_000, "{ordered} graphs without a cycle");
        assert!(cyclic > 1_000, "{cyclic} graphs with a cycle");
    }
}
