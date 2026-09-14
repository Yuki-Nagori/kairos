//! 领域服务：无状态、纯逻辑；命令适配层只做参数注入与调度。

pub mod deformation;
pub mod dependencies;
pub mod derive;
pub mod digest;
pub mod dualdomain;
pub mod fill_preview;
pub mod gate_location;
pub mod geometry;
pub mod gmsh;
pub mod iges;
pub mod jobs;
pub mod material;
pub mod mesh_store;
pub mod meshing;
pub mod midplane;
pub mod moldingfoam;
pub mod operators;
pub mod paths;
pub mod process;
pub mod project;
pub mod render_mesh;
pub mod repair;
pub mod report_pptx;
pub mod results;
pub mod runners;
pub mod step;
pub mod system;
pub mod thickness;
pub mod vec3;
pub mod vm;
pub mod volume_field;
pub mod workspace;

#[cfg(test)]
mod gap_tests;
