//! 领域服务：无状态、纯逻辑；命令适配层只做参数注入与调度。

pub mod deformation;
pub mod dependencies;
pub mod derive;
pub mod digest;
pub mod doe;
pub mod downloads;
pub mod dualdomain;
pub mod fill_preview;
pub mod gate_location;
pub mod geometry;
pub mod gmsh;
pub mod host;
pub mod iges;
pub mod job_lifecycle;
pub mod jobs;
pub mod material;
pub mod material_curve;
pub mod mesh_store;
pub mod meshing;
pub mod midplane;
pub mod moldingfoam;
pub mod operators;
pub mod optimize;
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
pub mod vm_run;
pub mod volume_field;
pub mod workspace;

#[cfg(test)]
mod gap_tests;
