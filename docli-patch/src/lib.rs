//! OOXML patching engine for docli.

pub mod id_alloc;
pub mod images;
pub mod normalize;
pub mod ops;
pub mod part_graph;
pub mod relationships;
pub mod run_split;
pub mod runs;
pub mod tables;

pub use id_alloc::IdAllocator;
pub use part_graph::{PartData, PartGraph};
pub use run_split::{split_runs_at_offsets, RunFragment, SplitResult};
