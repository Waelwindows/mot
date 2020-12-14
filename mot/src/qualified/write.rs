use super::*;

use std::io;
use std::io::SeekFrom;

impl FrameData {
    fn get_max_keyframe(&self) -> u16 {
        use FrameData::*;
        let res = match self {
            None | Pose(_) => 0u16,
            Linear(l) => l.iter().map(|x| x.frame as u16).max().unwrap_or(0),
            Smooth(l) => l.iter().map(|x| x.keyframe.frame as u16).max().unwrap_or(0),
        };
        res + 1
    }
    pub(crate) fn get_bits(&self) -> u8 {
        use FrameData::*;
        match self {
            None => 0,
            Pose(_) => 1,
            Linear(l) => 2,
            Smooth(l) => 3,
        }
    }
}

impl Vec3 {
    pub fn write<'a, W: io::Write + io::Seek>(&'a self, mut writer: W) -> io::Result<usize> {
        self.x.write()(&mut writer)?;
        self.y.write()(&mut writer)?;
        self.z.write()(&mut writer)?;
        Ok(0)
    }
    fn get_max_keyframe(&self) -> u16 {
        use std::cmp::max;
        let x = self.x.get_max_keyframe();
        let y = self.x.get_max_keyframe();
        let z = self.x.get_max_keyframe();
        max(max(x, y), z)
    }
    pub(crate) fn get_bits(&self) -> Vec<u8> {
        let x = self.x.get_bits();
        let y = self.x.get_bits();
        let z = self.x.get_bits();
        vec![x, y, z]
    }
}

impl BoneAnim {
    pub fn write<'a, W: io::Write + io::Seek>(&'a self) -> impl Fn(W) -> io::Result<usize> + 'a {
        use BoneAnim::*;
        move |mut writer| {
            match self {
                Rotation(v) => {
                    v.write(&mut writer)?;
                }
                Type1(v0, v1) => {
                    v0.write(&mut writer)?;
                    v1.write(&mut writer)?;
                } //rotationnown
                Position(v) => {
                    v.write(&mut writer)?;
                }
                PositionRotation {
                    //Type3
                    position,
                    rotation,
                } => {
                    position.write(&mut writer)?;
                    rotation.write(&mut writer)?;
                }
                RotationIK {
                    //Type4
                    rotation, // guess
                    target,
                } => {
                    target.write(&mut writer)?;
                    rotation.write(&mut writer)?;
                }
                ArmIK {
                    //Type 5
                    target,
                    rotation, // guess
                } => {
                    target.write(&mut writer)?;
                    rotation.write(&mut writer)?;
                }
                PositionIKRotation {
                    //Type 6
                    position, // guess
                    target,
                    // rotation,
                } => {
                    position.write(&mut writer)?;
                    target.write(&mut writer)?;
                    // rotation.write(&mut writer)?;
                }
            }
            Ok(0)
        }
    }
    fn get_max_keyframe(&self) -> u16 {
        use std::cmp::max;
        use BoneAnim::*;
        match self {
            Rotation(v) => v.get_max_keyframe(),
            Type1(v0, v1) => max(v0.get_max_keyframe(), v1.get_max_keyframe()), //rotationnown
            Position(v) => v.get_max_keyframe(),
            PositionRotation { position, rotation } => {
                max(position.get_max_keyframe(), rotation.get_max_keyframe())
            }
            RotationIK { rotation, target } => {
                max(rotation.get_max_keyframe(), target.get_max_keyframe())
            }
            ArmIK { rotation, target } => {
                max(rotation.get_max_keyframe(), target.get_max_keyframe())
            }
            PositionIKRotation { position, target } => {
                max(position.get_max_keyframe(), target.get_max_keyframe())
            }
        }
    }
    pub(crate) fn get_bits(&self) -> Vec<u8> {
        use BoneAnim::*;
        match self {
            Rotation(v) => v.get_bits(),
            Type1(v0, v1) => {
                let mut vec = v0.get_bits();
                vec.append(&mut v1.get_bits());
                vec
            } //rotationnown
            Position(v) => v.get_bits(),
            PositionRotation { position, rotation } => {
                let mut vec = position.get_bits();
                vec.append(&mut rotation.get_bits());
                vec
            }
            RotationIK { rotation, target } => {
                let mut vec = target.get_bits();
                vec.append(&mut rotation.get_bits());
                vec
            }
            ArmIK { rotation, target } => {
                let mut vec = target.get_bits();
                vec.append(&mut rotation.get_bits());
                vec
            }
            PositionIKRotation { position, target } => {
                let mut vec = position.get_bits();
                vec.append(&mut target.get_bits());
                // vec.append(&mut rotation.get_bits());
                vec
            }
        }
    }
    pub(crate) fn set_count(&self) -> usize {
        use BoneAnim::*;
        match self {
            Rotation(v) => 3,
            Type1(v0, v1) => 6, //rotationnown
            Position(v) => 3,
            PositionRotation { position, rotation } => 6,
            RotationIK { rotation, target } => 6,
            ArmIK { rotation, target } => 6,
            PositionIKRotation { position, target } => 6,
        }
    }
    pub(crate) fn sets(self) -> Vec<FrameData> {
        use BoneAnim::*;
        match self {
            Rotation(v) => vec![v.x, v.y, v.z],
            Type1(v0, v1) => vec![v0.x, v0.y, v0.z, v1.x, v1.y, v1.z], //rotationnown
            Position(v) => vec![v.x, v.y, v.z],
            PositionRotation { position, rotation } => vec![
                position.x, position.y, position.z, rotation.x, rotation.y, rotation.z,
            ],
            RotationIK { rotation, target } => vec![
                target.x, target.y, target.z, rotation.x, rotation.y, rotation.z,
            ], //rotationnown
            ArmIK { rotation, target } => vec![
                target.x, target.y, target.z, rotation.x, rotation.y, rotation.z,
            ], //rotationnown
            PositionIKRotation { position, target } => vec![
                position.x, position.y, position.z, target.x, target.y, target.z,
            ], //rotationnown
        }
    }
}

impl QualifiedMotion {
    pub(crate) fn get_bits(&self) -> Vec<u8> {
        let sets: Vec<FrameData> = self
            .anims
            .iter()
            .filter_map(|(_, a)| a.clone())
            .flat_map(|x| x.sets())
            .collect();
        sets.chunks(4)
            .map(|i| {
                i[0].as_bits()
                    | (i.get(1).map(FrameData::as_bits).unwrap_or(0) << 2)
                    | (i.get(2).map(FrameData::as_bits).unwrap_or(0) << 4)
                    | (i.get(3).map(FrameData::as_bits).unwrap_or(0) << 6)
            })
            .collect()
    }
    fn get_max_keyframe(&self) -> u16 {
        use std::cmp::max;
        use BoneAnim::*;
        self.anims
            .iter()
            .map(|(_, a)| a.clone().map(|a| a.get_max_keyframe()).unwrap_or(0))
            .max()
            .unwrap_or(1)
    }
    pub fn write<W: io::Write + io::Seek>(&self, mut writer: W) -> io::Result<usize> {
        use crate::const_table::*;

        let begin = writer.stream_position()?;
        writer.write(&32u32.to_le_bytes())?;
        writer.write(&36u32.to_le_bytes())?;
        writer.write(&[0; 16 + 8])?;
        let len: usize = self
            .anims
            .iter()
            .map(|(_, a)| a.clone().map(|a| a.set_count()).unwrap_or(0))
            .sum();
        //Have to add a terminal set
        let len = len as u16 + 1;
        println!("set len: {}", len);
        writer.write(&(len + 1 + 0x3FFF).to_le_bytes())?;
        writer.write(&self.get_max_keyframe().to_le_bytes())?;
        writer.write(&self.get_bits())?;
        let cur = writer.stream_position()? as usize;
        writer.write(&vec![0; cur % 6])?;
        let set_off = writer.stream_position()?;
        for (_, anim) in &self.anims {
            match anim {
                Some(anim) => anim.write()(&mut writer)?,
                None => continue,
            };
        }
        let bones_off = writer.stream_position()?;
        for (id, _) in &self.anims {
            // let id = crate::const_table::BONE_IDS[&bone.name[..]];
            writer.write(&(*id as u16).to_le_bytes())?;
        }
        let end = writer.stream_position()?;
        writer.seek(SeekFrom::Start(begin + 8))?;
        writer.write(&(set_off as u32).to_le_bytes())?;
        writer.write(&(bones_off as u32).to_le_bytes())?;
        let pad = writer.stream_position()? % 4;
        writer.write(&vec![0u8; pad as usize])?;
        Ok((end - begin) as usize)
    }
}
