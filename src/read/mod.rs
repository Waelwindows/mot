use nom::number::Endianness;
use nom::IResult;
use nom_ext::*;

use super::*;

// mod utilities;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum SetType {
    None,
    Pose,
    Linear,
    Smooth,
}

pub trait DeserializeContext: Sized {
    type Context;

    fn parse(i: &[u8], endian: Endianness, ctx: Self::Context) -> IResult<&[u8], Self>;
}

pub trait DeserializeEndian: Sized {
    fn parse(i: &[u8], endian: Endianness) -> IResult<&[u8], Self>;
}

impl<D: Default, T: DeserializeContext<Context = D>> DeserializeEndian for T {
    fn parse(i: &[u8], endian: Endianness) -> IResult<&[u8], Self> {
        Self::parse(i, endian, D::default())
    }
}

impl SetType {
    fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0b00u8 => Some(Self::None),
            0b01u8 => Some(Self::Pose),
            0b10u8 => Some(Self::Linear),
            0b11u8 => Some(Self::Smooth),
            _ => None,
        }
    }
    fn parse(i: (&[u8], usize)) -> IResult<(&[u8], usize), Self> {
        use nom::bits::complete::*;
        let (_, bits) = take(2usize)(i)?;
        let (i, c) = i;
        let i0 = if c == 0 { (&i[1..], 6) } else { (i, c - 2) };
        // println!("{:?}", bits);
        Ok((i0, Self::from_bits(bits).unwrap()))
    }
    fn parse_multi<'a>(counts: usize) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], Vec<Self>> {
        use nom::bits::bits;
        use nom::multi::count;
        bits(move |(i, _): (&'a [u8], usize)| count(SetType::parse, counts)((i, 6)))
    }
    fn parse_multi_legacy(count: usize) -> impl Fn(&[u8]) -> IResult<&[u8], Vec<Self>> {
        // for ( int i = 0, b = 0; i < keySetCount; i++ )
        // {
        //     if ( i % 8 == 0 )
        //         b = reader.ReadUInt16();

        //     KeySets.Add( new KeySet { Type = ( KeySetType ) ( ( b >> ( i % 8 * 2 ) ) & 3 ) } );
        // }
        use nom::number::complete::*;
        move |i0: &[u8]| {
            let mut b = 0;
            let mut sets = vec![];
            let mut i1 = i0;
            for i in 0..count {
                if i % 8 == 0 {
                    let (i2, b1) = le_u16(i1)?;
                    // println!("{:#b}", b1);
                    i1 = i2;
                    b = b1;
                }
                let val = Self::from_bits(((b >> (i % 8 * 2)) & 3) as u8).unwrap();
                sets.push(val);
            }
            Ok((i1, sets))
        }
    }
    fn read<'a>(&self, i0: &'a [u8]) -> IResult<&'a [u8], FrameData> {
        use nom::combinator::map;
        use nom::number::complete::*;

        match self {
            Self::None => Ok((i0, FrameData::None)),
            Self::Pose => map(le_f32, FrameData::Pose)(i0),
            Self::Linear => map(parse_linear(Endianness::Little), FrameData::Linear)(i0),
            Self::Smooth => map(parse_smooth(Endianness::Little), FrameData::Smooth)(i0),
        }
    }
}

//TODO: merge these 2 functions
fn parse_linear(endian: Endianness) -> impl Fn(&[u8]) -> IResult<&[u8], Vec<Keyframe>> {
    use nom::multi::*;
    use nom::number::complete::*;
    move |i0: &[u8]| {
        let (i, c) = u32_usize(endian)(i0)?;
        let (i, frames) = count(le_u16, c)(i)?;
        //Align at 4th byte
        let i = &i[i.len() % 4..];
        let (i, values) = count(le_f32, c)(i)?;
        let keyframes = frames
            .into_iter()
            .zip(values.into_iter())
            .map(|(frame, value)| Keyframe { frame, value })
            .collect();
        Ok((i, keyframes))
    }
}
fn parse_smooth(_endian: Endianness) -> impl Fn(&[u8]) -> IResult<&[u8], Vec<InterpKeyframe>> {
    use nom::combinator::map;
    use nom::multi::*;
    use nom::number::complete::*;
    use nom::sequence::pair;

    move |i0: &[u8]| {
        let (i, c) = map(le_u16, |n| n as usize)(i0)?;
        // println!("count={} left={}", c, i.len());
        let (i, frames) = count(le_u16, c)(i)?;
        //Align at 4th byte
        // println!("{} - {}={}", i.len(), i.len() % 4, i.len() - i.len() % 4);
        let i = &i[i.len() % 4..];
        let (i, values) = count(pair(le_f32, le_f32), c)(i)?;
        let keyframes = frames
            .into_iter()
            .zip(values.into_iter())
            .map(|(frame, (value, interpolation))| InterpKeyframe {
                keyframe: Keyframe { frame, value },
                interpolation,
            })
            .collect();
        Ok((i, keyframes))
    }
}

impl DeserializeEndian for Motion {
    fn parse(i: &[u8], endian: Endianness) -> IResult<&[u8], Self> {
        use nom::combinator::map;
        use nom::number::complete::*;

        let (i0, count) = offset_then(i, map(le_u16, |n| (n & 0x3FFF) as usize), endian)(i)?;
        println!("Set count: {}", count);
        let (i0, types) = offset_then(i, SetType::parse_multi(count), endian)(i0)?;
        let (i0, ks_offset) = u32_usize(endian)(i0)?;
        let (i0, bones) = offset_then(i, many_until_nth(le_u16, 0, 1), endian)(i0)?;

        // let mut sets = vec![];
        // i0 = &i[ks_offset..];
        // for (idx, stype) in types.iter().enumerate() {
        //     println!("#{} {:?} len={}", idx, stype, i0.len());
        //     let (i1, val) = stype.read(i0)?;
        //     i0 = i1;
        //     sets.push(val);
        // }
        // println!("iter");
        let sets1: Vec<FrameData> = types
            .iter()
            .scan(&i[ks_offset..], |i1, st| match st.read(i1) {
                Ok((i, v)) => {
                    *i1 = i;
                    Some(v)
                }
                _ => None,
            })
            .collect();
        // println!("{:?}", sets1[1]);
        // assert_eq!(sets, sets1);

        Ok((
            i0,
            Motion {
                bones: bones.into_iter().map(Into::into).collect(),
                sets: sets1,
            },
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const DB: &[u8] = include_bytes!("../../assets/bone_data.bin");
    const INPUT: &[u8] = include_bytes!("../../assets/mot_PV001.bin");

    #[test]
    fn set_read() {
        use nom::multi::count;

        let i = &INPUT[0x24..];
        let (_, val) = count(SetType::parse, 520)((i, 6)).unwrap();
        let (_, val1) = SetType::parse_multi_legacy(520)(i).unwrap();
        // assert_eq!(val, SetType::Smooth)
        // for vals in val.chunks(8) {
        //     println!("{:?}", vals);
        // }
        assert_eq!(val, val1);
    }
    #[test]
    fn set_user_read() {
        use self::SetType::*;

        use nom::multi::count;

        let i = &[0b11_10_01_00u8, 0b11_00_01_11][..];
        let val = count(SetType::parse, 8)((i, 6)).unwrap().1;
        assert_eq!(
            val,
            [None, Pose, Linear, Smooth, Smooth, Pose, None, Smooth]
        )
    }
    #[test]
    fn motion_test() {
        let (_, val) = Motion::parse(INPUT, Endianness::Little).unwrap();
        let len = val.sets.iter().filter(|x| **x != FrameData::None).count();
        assert_eq!(val.sets.len(), 583);
        assert_eq!(len, 137);

        println!(
            "pos is {}",
            val.bones.iter().position(|&r| r == 71).unwrap()
        );
    }
}
