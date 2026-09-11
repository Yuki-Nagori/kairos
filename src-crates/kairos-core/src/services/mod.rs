//! 领域服务：无状态、纯逻辑；命令适配层只做参数注入与调度。

pub mod dependencies;
pub mod derive;
pub mod dualdomain;
pub mod geometry;
pub mod gmsh;
pub mod iges;
pub mod jobs;
pub mod material;
pub mod meshing;
pub mod midplane;
pub mod openfoam;
pub mod operators;
pub mod process;
pub mod project;
pub mod repair;
pub mod results;
pub mod runners;
pub mod step;
pub mod system;
pub mod vm;

#[cfg(test)]
mod gap_tests;
