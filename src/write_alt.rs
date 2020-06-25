use super::*;

use std::io::{Seek, SeekFrom};

impl Motion {
    pub(crate) fn get_bits(&self) -> Vec<u8> {
        self.sets
            .chunks(4)
            .map(|i| {
                i[0].as_bits()
                    | (i.get(1).map(FrameData::as_bits).unwrap_or(0) << 2)
                    | (i.get(2).map(FrameData::as_bits).unwrap_or(0) << 4)
                    | (i.get(3).map(FrameData::as_bits).unwrap_or(0) << 6)
            })
            .collect()
    }

    fn get_max_keyframe(&self) -> u16 {
        use FrameData::*;
        self.sets
            .iter()
            .map(|x| match x {
                None | Pose(_) => 0u16,
                Linear(l) => l.iter().map(|x| x.frame as u16).max().unwrap_or(0),
                Smooth(l) => l.iter().map(|x| x.keyframe.frame as u16).max().unwrap_or(0),
            })
            .max()
            .unwrap_or(0)
            + 1
    }
    pub fn write<'a, W: io::Write + io::Seek>(&'a self) -> impl Fn(W) -> io::Result<usize> + 'a {
        move |mut writer| {
            let begin = writer.stream_position()?;
            writer.write(&32u32.to_le_bytes())?;
            writer.write(&36u32.to_le_bytes())?;
            writer.write(&[0; 16+8])?;
            let len = self.sets.len() as u16;
            println!("set len {}", len);
            // let len  = len + len % 2;
            // println!("pre-len {}", len);
            writer.write(&(len + 1 + 0x3FFF).to_le_bytes())?;
            writer.write(&self.get_max_keyframe().to_le_bytes())?;
            writer.write(&self.get_bits())?;
            let cur = writer.stream_position()? as usize;
            writer.write(&vec![0; cur % 6])?;
            let set_off = writer.stream_position()?;
            for set in &self.sets {
                set.write()(&mut writer)?;
            }
            let bones_off = writer.stream_position()?;
            for bone in &self.bones {
                writer.write(&(*bone as u16).to_le_bytes())?;
            }
            let end = writer.stream_position()?;
            writer.seek(SeekFrom::Start(begin+8))?;
            writer.write(&(set_off as u32).to_le_bytes())?;
            writer.write(&(bones_off as u32).to_le_bytes())?;
            Ok(((end-begin) as usize))
        }
    }
}

use std::io;

impl FrameData {
    fn as_bits(&self) -> u8 {
        use FrameData::*;
        match self {
            None => 0,
            Pose(_) => 1,
            Linear(_) => 2,
            Smooth(_) => 3,
        }
    }
    pub fn write<'a, W: io::Write + io::Seek>(&'a self) -> impl Fn(W) -> io::Result<usize> + 'a {
        move |mut writer| {
            match self {
                Self::Pose(p) => writer.write(&p.to_le_bytes()),
                Self::Linear(v) => {
                    writer.write(&(v.len() as u16).to_le_bytes())?;
                    for frame in v {
                        writer.write(&frame.frame.to_le_bytes())?;
                    }
                    let pos = writer.stream_position()?;
                    writer.write(&vec![0; pos as usize % 4])?;
                    for frame in v {
                        writer.write(&frame.value.to_le_bytes())?;
                    }
                    Ok(0)
                },
                Self::Smooth(v) => {
                    writer.write(&(v.len() as u16).to_le_bytes())?;
                    for frame in v {
                        writer.write(&frame.keyframe.frame.to_le_bytes())?;
                    }
                    let pos = writer.stream_position()?;
                    writer.write(&vec![0; pos as usize % 4])?;
                    for frame in v {
                        writer.write(&frame.keyframe.value.to_le_bytes())?;
                        writer.write(&frame.interpolation.to_le_bytes())?;
                    }
                    Ok(0)
                }
                _ => Ok(0)
            }
        }
    }
}