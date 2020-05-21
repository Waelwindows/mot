use nom::number::Endianness;
use nom::IResult;
use nom::{map, u32};

use core::ops::Index;
use core::ops::Range;
use std::slice::SliceIndex;

pub(crate) fn usize(endian: Endianness) -> impl FnOnce(&[u8]) -> IResult<&[u8], usize> {
    move |i: &[u8]| map!(i, u32!(endian), |n| n as usize)
}

pub(crate) fn read_at<'a, O, F>(
    i0: &'a [u8],
    f: F,
    endian: Endianness,
) -> impl FnOnce(&'a [u8]) -> IResult<&'a [u8], O>
where
    F: FnOnce(&'a [u8]) -> IResult<&'a [u8], O>,
{
    move |i: &[u8]| {
        let (i1, offset) = usize(endian)(i)?;
        f(&i0[offset..]).map(|(_, v)| (i1, v))
    }
}

use nom::error::ParseError;
// #[cfg(feature = "alloc")]
pub fn many_until<I, O, E, F>(f: F, v: O) -> impl Fn(I) -> IResult<I, Vec<O>, E>
where
    I: Clone,
    O: PartialEq,
    F: Fn(I) -> IResult<I, O, E>,
    E: ParseError<I>,
{
    move |i: I| {
        let mut res = Vec::new();
        let mut i = i.clone();
        loop {
            let (i1, val) = f(i.clone())?;
            i = i1;
            if val == v {
                break;
            } else {
                res.push(val);
            }
        }
        Ok((i, res))
    }
}

pub fn many_until_nth<I, O, E, F>(
    f: F,
    v: O,
    occurance: usize,
) -> impl Fn(I) -> IResult<I, Vec<O>, E>
where
    I: Clone,
    O: PartialEq,
    F: Fn(I) -> IResult<I, O, E>,
    E: ParseError<I>,
{
    move |i: I| {
        let mut res = Vec::new();
        let mut i = i.clone();
        let mut o = 1;
        loop {
            let (i1, val) = f(i.clone())?;
            i = i1;
            if val == v {
                if o <= occurance {
                    o += 1;
                    res.push(val);
                } else {
                    break;
                }
            } else {
                res.push(val);
            }
        }
        Ok((i, res))
    }
}
