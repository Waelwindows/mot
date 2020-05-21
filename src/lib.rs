pub mod read;
mod write;

pub struct Motion {
    pub sets: Vec<FrameData>,
    pub bones: Vec<usize>,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum FrameData {
    None,
    Pose(f32),
    Linear(Vec<Keyframe>),
    Smooth(Vec<InterpKeyframe>),
}

#[derive(Debug, Default, PartialEq, PartialOrd, Clone)]
pub struct Keyframe {
    pub frame: u16,
    pub value: f32,
}

#[derive(Debug, Default, PartialEq, PartialOrd, Clone)]
pub struct InterpKeyframe {
    pub keyframe: Keyframe,
    pub interpolation: f32,
}
