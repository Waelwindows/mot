use super::*;

mod write;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct QualifiedMotion {
    pub anims: Vec<(usize, Option<BoneAnim>)>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Vec3 {
    pub x: FrameData,
    pub y: FrameData,
    pub z: FrameData,
}

impl Default for Vec3 {
    fn default() -> Self {
        let x = FrameData::None;
        let y = FrameData::None;
        let z = FrameData::None;
        Self { x, y, z }
    }
}

impl Vec3 {
    pub const ZERO: Self = Vec3 {
        x: FrameData::Pose(0.),
        y: FrameData::Pose(0.),
        z: FrameData::Pose(0.),
    };

    pub fn to_pose(&mut self) {
        use FrameData::*;
        let poseify = |x: &FrameData| match x {
            None => None,
            Pose(a) => Pose(*a),
            Linear(l) => Pose(l[0].value),
            Smooth(l) => Pose(l[0].keyframe.value),
        };
        self.x = poseify(&self.x);
        self.y = poseify(&self.y);
        self.z = poseify(&self.z);
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum BoneAnim {
    Rotation(Vec3),
    Type1(Vec3, Vec3), //unknown
    Position(Vec3),
    PositionRotation {
        //Type3
        position: Vec3,
        rotation: Vec3,
    },
    RotationIK {
        //Type4; target is first
        target: Vec3,
        rotation: Vec3, // guess
    },
    ArmIK {
        //Type 5
        target: Vec3,
        rotation: Vec3,
    },
    PositionIKRotation {
        //Type 6
        position: Vec3, // guess
        target: Vec3,
        // rotation: Vec3,
    },
}

use diva_db::bone::*;
use diva_db::mot::*;
use std::collections::VecDeque;

impl Motion {
    pub fn qualify<'a>(
        self,
        mot_db: &MotionSetDatabase,
        bone_db: &BoneDatabase<'a>,
    ) -> QualifiedMotion {
        let mut sets: VecDeque<FrameData> = self.sets.into();
        let bones = &bone_db.skeletons[0].bones;
        let mut vec3 = || {
            let x = sets.pop_front().unwrap();
            let y = sets.pop_front().unwrap();
            let z = sets.pop_front().unwrap();
            Vec3 { x, y, z }
        };
        let mut anims = vec![];
        for id in self.bones {
            let name = &mot_db.bones[id];
            let bone = bone_db.skeletons[0]
                .bones
                .iter()
                .find(|x| &x.name[..] == name);
            let bone = match bone {
                Some(b) => b.clone(),
                None if name == "gblctr" => Bone {
                    mode: BoneType::Position,
                    ..Default::default()
                },
                None if name == "kg_ya_ex" => Bone {
                    mode: BoneType::Rotation,
                    ..Default::default()
                },
                None => {
                    anims.push((id, None));
                    continue;
                }
            };
            let mode = bone.mode;
            anims.push((
                id,
                Some(match mode {
                    BoneType::Rotation => BoneAnim::Rotation(vec3()),
                    BoneType::Type1 => {
                        println!("Weird stuff inbound, encoutered TYPE1");
                        BoneAnim::Type1(vec3(), vec3())
                    }
                    BoneType::Position => BoneAnim::Position(vec3()),
                    BoneType::Type3 => BoneAnim::PositionRotation {
                        position: vec3(),
                        rotation: vec3(),
                    },
                    BoneType::Type4 => BoneAnim::RotationIK {
                        target: vec3(),
                        rotation: vec3(),
                    },
                    BoneType::Type5 => BoneAnim::ArmIK {
                        target: vec3(),
                        rotation: vec3(),
                    },
                    BoneType::Type6 => BoneAnim::PositionIKRotation {
                        position: vec3(),
                        target: vec3(),
                    },
                }),
            ))
        }
        println!("{} set(s) are left", sets.len());
        QualifiedMotion { anims }
    }
}

impl QualifiedMotion {
    ///Order animations according to DIVA's ordering
    ///
    ///Ordering normally does work in DEBUG, but breaks expressions and fingers in PV
    ///In order to fix this, the anims must be sorted in DIVA's order
    pub fn sort(&mut self, motset_db: &MotionSetDatabase) {
        use crate::const_table::*;
        self.anims.sort_by(|x, y| {
            BONE_IDS
                .get(&motset_db.bones[x.0][..])
                .unwrap_or(&255)
                .cmp(&BONE_IDS.get(&motset_db.bones[y.0][..]).unwrap_or(&255))
        });
    }
}
