use cookie_factory::bytes::*;
use cookie_factory::combinator::*;
use cookie_factory::multi::*;
use cookie_factory::sequence::tuple;
use cookie_factory::*;

use super::*;

use std::io;

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

    pub(crate) fn write<'a, W: io::Write + 'a>(&'a self) -> impl SerializeFn<W> + 'a {
        use cookie_factory::bytes::*;
        let frames = self.sets.iter().map(FrameData::write);
        let bones = self.bones.iter().map(|x| le_u16(*x as u16));
        let len = self.sets.len() as u16;
        let len  = len + len % 2;
        dbg!(self.sets.len());
        dbg!(len);
        dbg!(self.get_bits());
        tuple((
            le_u16(len + 0x3FFF),
            le_u16(self.get_max_keyframe()),
            slice(self.get_bits()),
            pad(4),
            all(frames),
            all(bones),
        ))
    }

    pub fn pub_write<'a, W: BackToTheBuffer + 'a>(&'a self) -> impl SerializeFn<W> + 'a {
        back_to_the_buffer(
            32,
            move |out: WriteContext<W>| gen(self.write(), out),
            move |out, len| {
                gen_simple(
                    tuple((
                        le_u32(32),
                        le_u32(36),
                        le_u32(36 + self.sets.len() as u32 / 4 + 1),
                        le_u32(len as u32 - self.bones.len() as u32 * 2 + 32),
                        slice(vec![0; 16]),
                    )),
                    out,
                )
            },
        )
    }
}

fn pad<W: io::Write>(padding: usize) -> impl SerializeFn<W> {
    move |out: WriteContext<W>| {
        let pos = out.position as usize;
        let pad = pos % padding;
        println!("pad#: {}: cur: {}, pad {},  total {}", padding, out.position, pad, out.position + pad as u64);
        slice(vec![0; pad])(out)
    }
}

const EMPTY_LINEAR: &Vec<Keyframe> = &vec![];
const EMPTY_SMOOTH: &Vec<InterpKeyframe> = &vec![];

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
    fn write<'a, W: io::Write + 'a>(&'a self) -> impl SerializeFn<W> + 'a {
        let pose = match self {
            FrameData::Pose(_) => true,
            _ => false,
        };
        let linear = match self {
            FrameData::Linear(_) => true,
            _ => false,
        };
        let smooth = match self {
            FrameData::Smooth(_) => true,
            _ => false,
        };
        tuple((
            cond(pose, self.write_pose()),
            cond(linear, self.write_linear()),
            cond(smooth, self.write_smooth()),
        ))
    }
    fn write_pose<W: io::Write>(&self) -> impl SerializeFn<W> {
        let pose = match self {
            FrameData::Pose(p) => Some(*p),
            _ => None,
        };
        cond(pose.is_some(), le_f32(pose.unwrap_or(0.)))
    }
    fn write_linear<'a, W: io::Write + 'a>(&'a self) -> impl SerializeFn<W> + 'a {
        let linear = match self {
            FrameData::Linear(l) => Some(l),
            _ => None,
        };
        let linear = linear.unwrap_or_else(|| EMPTY_LINEAR);
        let frames = linear.iter().map(|x| le_u16(x.frame));
        let values = linear.iter().map(|x| le_f32(x.value));
        tuple((
            le_u16(linear.len() as u16),
            all(frames),
            pad(4),
            all(values),
        ))
    }
    fn write_smooth<'a, W: io::Write + 'a>(&'a self) -> impl SerializeFn<W> + 'a {
        let linear = match self {
            FrameData::Smooth(l) => Some(l),
            _ => None,
        };
        let linear = linear.unwrap_or_else(|| EMPTY_SMOOTH);
        let frames = linear.iter().map(|x| le_u16(x.keyframe.frame));
        let values = linear
            .iter()
            .map(|x| tuple((le_f32(x.keyframe.value), le_f32(x.interpolation))));
        tuple((
            le_u16(linear.len() as u16),
            all(frames),
            pad(4),
            all(values),
        ))
    }
}
