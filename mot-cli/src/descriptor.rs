use mot::qualified;
use mot::FrameData;
use serde::{Serialize, Deserialize};

use std::collections::HashMap;
use std::path::PathBuf;

#[serde(default)] 
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Config {
    pub default: Vec<String>,
    pub custom: HashMap<String, BoneDescriptor>,
    pub mot_db_path: Option<PathBuf>,
    pub bone_db_path: Option<PathBuf>,
}

#[derive(Default, Serialize, Deserialize, Debug)]
#[serde(default)] 
pub struct BoneDescriptor {
    pub position: Option<Vec3>,
    pub rotation: Option<Vec3>,
    pub target: Option<Vec3>,
}

#[derive(Default, Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl From<Vec3> for qualified::Vec3 {
    fn from(vec: Vec3) -> Self {
        let Vec3(x, y, z) = vec;
        let x = if x.is_nan() { FrameData::None } else { FrameData::Pose(x) };
        let y = if y.is_nan() { FrameData::None } else { FrameData::Pose(y) };
        let z = if z.is_nan() { FrameData::None } else { FrameData::Pose(z) };
        Self { x, y, z }
    }
}
