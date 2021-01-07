extern crate asexp;
extern crate petgraph;

use asexp::atom::Atom;
use asexp::token::{Token, Tokenizer};
use asexp::Sexp;
use petgraph::graph::NodeIndex;
use petgraph::{Directed, Graph};
use std::collections::BTreeMap;

pub fn parse_gml<NodeWeightFn, EdgeWeightFn, N, E>(
    s: &str,
    node_weight_fn: &NodeWeightFn,
    edge_weight_fn: &EdgeWeightFn,
) -> Result<Graph<N, E, Directed>, &'static str>
where
    NodeWeightFn: Fn(Option<&Sexp>) -> Option<N>,
    EdgeWeightFn: Fn(Option<&Sexp>) -> Option<E>,
{
    match parse_gml_to_sexp(s) {
        Ok(sexp) => sexp_to_graph(sexp, node_weight_fn, edge_weight_fn),
        Err(_) => Err("Invalid GML"),
    }
}

fn parse_gml_to_sexp(s: &str) -> Result<Sexp, ()> {
    let iter = Tokenizer::new(s, true).with_curly_around();
    let iter = iter.map(|t| match t {
        Token::OpenBracket => Token::OpenCurly,
        Token::CloseBracket => Token::CloseCurly,
        a => a,
    });

    Sexp::parse_iter(iter)
}

fn sexp_to_graph<NodeWeightFn, EdgeWeightFn, N, E>(
    sexp: Sexp,
    node_weight_fn: &NodeWeightFn,
    edge_weight_fn: &EdgeWeightFn,
) -> Result<Graph<N, E, Directed>, &'static str>
where
    NodeWeightFn: Fn(Option<&Sexp>) -> Option<N>,
    EdgeWeightFn: Fn(Option<&Sexp>) -> Option<E>,
{
    let mut map = sexp.into_map()?;

    if let Some(Sexp::Map(v)) = map.remove("graph") {
        let mut node_map: BTreeMap<u64, NodeIndex> = BTreeMap::new();
        let mut graph = Graph::new();
        let mut edges = Vec::new();

        for (k, v) in v {
            match k.get_str() {
                Some("directed") => match v.get_uint() {
                    Some(1) => {}
                    _ => {
                        return Err("only directed graph supported");
                    }
                },
                Some("node") => {
                    let node_info = v.into_map()?;
                    if let Some(&Sexp::Atom(Atom::UInt(node_id))) = node_info.get("id") {
                        match node_weight_fn(node_info.get("weight")) {
                            Some(weight) => {
                                let idx = graph.add_node(weight);
                                if node_map.insert(node_id, idx).is_some() {
                                    return Err("duplicate node-id");
                                }
                            }
                            None => {
                                return Err("invalid node weight");
                            }
                        }
                    } else {
                        return Err("Invalid id");
                    }
                }
                Some("edge") => {
                    let edge_info = v.into_map()?;

                    let source =
                        if let Some(&Sexp::Atom(Atom::UInt(source))) = edge_info.get("source") {
                            source
                        } else {
                            return Err("Invalid source id");
                        };

                    let target =
                        if let Some(&Sexp::Atom(Atom::UInt(target))) = edge_info.get("target") {
                            target
                        } else {
                            return Err("Invalid target id");
                        };

                    match edge_weight_fn(edge_info.get("weight")) {
                        Some(weight) => {
                            edges.push((source, target, weight));
                        }
                        None => {
                            return Err("invalid edge weight");
                        }
                    }
                }
                _ => {
                    return Err("invalid item");
                }
            }
        }

        for (source, target, weight) in edges {
            let source_idx = node_map[&source];
            let target_idx = node_map[&target];
            graph.add_edge(source_idx, target_idx, weight);
        }

        Ok(graph)
    } else {
        Err("no graph given or invalid")
    }
}

#[test]
fn test_parse_gml() {
    let gml = "
    # comment
    graph
    [
        directed 1
        node
        [
          id 1
          \
               weight 1.0
        ]
        node
        [
          id 2
        ]
        edge
        \
               [
           source 1
           target 2
           weight 1.1000
        ]
        \
               edge
        [
           source 2
           target 1
        ]
    ]
    ";

    let g = parse_gml(
        gml,
        &|s| -> Option<f64> { Some(s.and_then(Sexp::get_float).unwrap_or(0.0)) },
        &|_| -> Option<()> { Some(()) },
    );
    assert!(g.is_ok());
    let g = g.unwrap();
    assert_eq!(true, g.is_directed());
    assert_eq!(
        true,
        g.find_edge(NodeIndex::new(0), NodeIndex::new(1)).is_some()
    );
    assert_eq!(
        true,
        g.find_edge(NodeIndex::new(1), NodeIndex::new(0)).is_some()
    );
    assert_eq!(Some(&1.0), g.node_weight(NodeIndex::new(0)));
    assert_eq!(Some(&0.0), g.node_weight(NodeIndex::new(1)));
}
