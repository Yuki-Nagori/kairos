//! 领域服务：无状态、纯逻辑；命令适配层只做参数注入与调度。

pub mod dependencies;
pub mod geometry;
pub mod gmsh;
pub mod jobs;
pub mod material;
pub mod meshing;
pub mod openfoam;
pub mod process;
pub mod project;
pub mod results;
pub mod runners;
pub mod system;
pub mod vm;

#[cfg(test)]
mod gap_tests;
