//! Map: cities, nodes, track edges, signals.
//!
//! The track is a graph. Nodes are points (stations, junctions, signal
//! positions). Edges are track segments with a length in metres. Trains move
//! along edges at a constant speed; signals live on edges and gate passage.
//!
//! Coordinates are in a 2D "world" frame (metres) for the client to render.
//! The engine only uses them for distance; all movement is along edges.

use serde::{Deserialize, Serialize};

/// A node in the track graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: u32,
    /// World position (metres). Client renders from this.
    pub pos: [f64; 2],
    /// The city this node belongs to, if it is a station.
    pub city: Option<u32>,
}

/// A track segment between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: u32,
    pub a: u32,
    pub b: u32,
    /// Length in metres (should match the node distance; stored for speed).
    pub length: f64,
    /// Whether this segment has a signal protecting it (one direction).
    pub has_signal: bool,
    /// True if this edge is a siding branch (a short spur off the main line).
    /// Siding edges are excluded from signal occupancy checks (ADR 0016).
    pub is_siding: bool,
}

/// A siding: a short dead-end branch track a train can pull onto so another
/// can pass. The player's main throughput tool (DESIGN.md §4, ADR 0016).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Siding {
    pub id: u32,
    /// The siding branch edge (the edge the train drives onto).
    pub edge: u32,
    /// The main-line node the siding branches from.
    pub node: u32,
    /// The siding dead-end node (where a diverted train parks).
    pub end_node: u32,
}

/// A city: a station hub with goods supply and demand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub id: u32,
    pub name: String,
    /// Station node id.
    pub station: u32,
    /// Goods this city produces (supply), per tick capacity.
    pub supply: Vec<(crate::Goods, u32)>,
    /// Goods this city wants (demand), per tick capacity.
    pub demand: Vec<(crate::Goods, u32)>,
}

/// The whole map: nodes + edges + cities + sidings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub id: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub cities: Vec<City>,
    /// Placed sidings (ADR 0016).
    pub sidings: Vec<Siding>,
}

impl Map {
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn edge(&self, id: u32) -> Option<&Edge> {
        self.edges.iter().find(|e| e.id == id)
    }
    /// Edges adjacent to a node.
    pub fn edges_at(&self, node: u32) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.a == node || e.b == node).collect()
    }
    /// Distance between two nodes (world frame).
    pub fn node_distance(&self, a: u32, b: u32) -> Option<f64> {
        let (na, nb) = (self.node(a)?, self.node(b)?);
        let dx = na.pos[0] - nb.pos[0];
        let dy = na.pos[1] - nb.pos[1];
        Some((dx * dx + dy * dy).sqrt())
    }
    /// The edge already connecting `a` and `b`, if any. Siding branch edges
    /// are ignored: a siding shares a main-line node with the main edge, and
    /// routing/movement should resolve to the main edge, not the spur.
    pub fn edge_between(&self, a: u32, b: u32) -> Option<&Edge> {
        self.edges
            .iter()
            .find(|e| (!e.is_siding) && ((e.a == a && e.b == b) || (e.b == a && e.a == b)))
    }
}

/// Grid cell size in metres (ADR 0004 placement grid; ADR 0015 build
/// commands). Labeled assumption: 300 m matches the default map's dominant
/// segment spacing (flagged in DESIGN.md §8).
pub const GRID_STEP: f64 = 300.0;

/// One grid step (in metres) for the 8-way direction:
/// 0=E 1=W 2=N 3=S 4=NE 5=NW 6=SE 7=SW (map y-axis points "north").
/// Diagonal steps move 300 m in each axis (300√2 m total).
pub fn grid_dir_step(dir: u8) -> Option<[f64; 2]> {
    let d = match dir {
        0 => [1.0, 0.0],
        1 => [-1.0, 0.0],
        2 => [0.0, 1.0],
        3 => [0.0, -1.0],
        4 => [1.0, 1.0],
        5 => [-1.0, 1.0],
        6 => [1.0, -1.0],
        7 => [-1.0, -1.0],
        _ => return None,
    };
    Some([d[0] * GRID_STEP, d[1] * GRID_STEP])
}

/// Find an existing node within `eps` metres of `pos`. Used so a build can
/// connect to a nearby node instead of creating a duplicate one.
pub fn node_near(map: &Map, pos: [f64; 2], eps: f64) -> Option<u32> {
    map.nodes
        .iter()
        .find(|n| {
            let d = ((n.pos[0] - pos[0]).powi(2) + (n.pos[1] - pos[1]).powi(2)).sqrt();
            d <= eps
        })
        .map(|n| n.id)
}

/// Build the default small puzzle map: 6 cities, ~14 nodes, a few signals.
/// Deterministic, hand-authored (no RNG), so tests can hard-code expectations.
pub fn default_map() -> Map {
    // World frame: roughly 2000 x 1400 m.
    let nodes: Vec<Node> = vec![
        // City A (north-west)
        Node { id: 0, pos: [150.0, 1150.0], city: Some(0) },
        Node { id: 1, pos: [450.0, 1150.0], city: None },
        Node { id: 2, pos: [750.0, 1150.0], city: None },
        // City B (north-east)
        Node { id: 3, pos: [1050.0, 1150.0], city: Some(1) },
        Node { id: 4, pos: [1350.0, 1150.0], city: None },
        Node { id: 5, pos: [1650.0, 1150.0], city: Some(2) }, // City C (far NE)
        // City D (south-west)
        Node { id: 6, pos: [150.0, 450.0], city: Some(3) },
        Node { id: 7, pos: [450.0, 450.0], city: None },
        Node { id: 8, pos: [750.0, 450.0], city: None },
        // City E (south-east)
        Node { id: 9, pos: [1050.0, 450.0], city: Some(4) },
        Node { id: 10, pos: [1350.0, 450.0], city: None },
        Node { id: 11, pos: [1650.0, 450.0], city: Some(5) }, // City F (far SE)
        // Central junction connecting the two lines
        Node { id: 12, pos: [750.0, 800.0], city: None },
        Node { id: 13, pos: [1050.0, 800.0], city: None },
    ];

    let edges: Vec<Edge> = vec![
        // North line
        edge(0, 0, 1, false),
        edge(1, 1, 2, true),   // signal
        edge(2, 2, 3, false),
        edge(3, 3, 4, false),
        edge(4, 4, 5, true),   // signal
        // South line
        edge(5, 6, 7, false),
        edge(6, 7, 8, true),   // signal
        edge(7, 8, 9, false),
        edge(8, 9, 10, false),
        edge(9, 10, 11, true), // signal
        // Cross connections (the "puzzle" part: limited capacity, signals)
        edge(10, 2, 12, true),  // north-mid to centre
        edge(11, 8, 12, false), // south-mid to centre
        edge(12, 12, 13, false),
        edge(13, 13, 3, true),  // centre to north-east
    ];

    // Goods: Coal, Steel, Food, Timber.
    use crate::Goods::*;
    let cities: Vec<City> = vec![
        City { id: 0, name: "A".into(), station: 0,
               supply: vec![(Coal, 4), (Timber, 2)], demand: vec![(Food, 2), (Steel, 1)] },
        City { id: 1, name: "B".into(), station: 3,
               supply: vec![(Steel, 3)], demand: vec![(Coal, 3), (Timber, 1)] },
        City { id: 2, name: "C".into(), station: 5,
               supply: vec![(Food, 4)], demand: vec![(Steel, 2), (Coal, 1)] },
        City { id: 3, name: "D".into(), station: 6,
               supply: vec![(Timber, 3), (Food, 2)], demand: vec![(Coal, 2), (Steel, 2)] },
        City { id: 4, name: "E".into(), station: 9,
               supply: vec![(Steel, 2), (Food, 2)], demand: vec![(Timber, 2), (Coal, 1)] },
        City { id: 5, name: "F".into(), station: 11,
               supply: vec![(Coal, 3)], demand: vec![(Food, 3), (Timber, 2)] },
    ];

    Map { id: "default".into(), nodes, edges, cities, sidings: vec![] }
}

fn edge(id: u32, a: u32, b: u32, has_signal: bool) -> Edge {
    Edge { id, a, b, length: 300.0, has_signal, is_siding: false }
}

/// The default signals: one per edge flagged `has_signal` in the default
/// map, protecting the direction the map author chose. Derived from the map
/// so `SimState::new_game` can build a full fresh state.
pub fn default_signals() -> Vec<crate::state::Signal> {
    use crate::state::Signal;
    vec![
        Signal { id: 0, edge: 1, from: 1, to: 2, green: true },
        Signal { id: 1, edge: 4, from: 4, to: 5, green: true },
        Signal { id: 2, edge: 6, from: 7, to: 8, green: true },
        Signal { id: 3, edge: 9, from: 10, to: 11, green: true },
        Signal { id: 4, edge: 10, from: 2, to: 12, green: true },
        Signal { id: 5, edge: 13, from: 13, to: 3, green: true },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_map_is_well_formed() {
        let m = default_map();
        assert_eq!(m.nodes.len(), 14);
        assert_eq!(m.edges.len(), 14);
        assert_eq!(m.cities.len(), 6);
        // Every edge endpoint must be a real node.
        for e in &m.edges {
            assert!(m.node(e.a).is_some());
            assert!(m.node(e.b).is_some());
        }
        // Every city station must be a real node.
        for c in &m.cities {
            assert!(m.node(c.station).is_some());
        }
    }

    #[test]
    fn node_distance_matches_stored_length() {
        let m = default_map();
        // Node 0 (150,1150) to node 1 (450,1150) = 300 m.
        assert!((m.node_distance(0, 1).unwrap() - 300.0).abs() < 1e-6);
    }
}
