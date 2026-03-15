//! OOXML patching engine for docli.

pub mod id_alloc;
pub mod normalize;
pub mod part_graph;
pub mod relationships;
pub mod run_split;
pub mod runs;

pub use id_alloc::IdAllocator;
pub use part_graph::{PartData, PartGraph};
pub use run_split::{split_runs_at_offsets, RunFragment, SplitResult};
