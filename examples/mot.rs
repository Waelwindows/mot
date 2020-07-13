use anyhow::*;
use bvh_anim::*;
use diva_db::bone::*;
use diva_db::mot::*;
use log::*;
use mot::*;
use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "mot",
    about = "ports over motions using bvh. made by: Waelwindows"
)]
struct Opt {
    #[structopt(parse(from_os_str))]
    mot_db: PathBuf,

    #[structopt(parse(from_os_str))]
    bone_db: PathBuf,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    #[structopt(parse(from_os_str))]
    bvh: PathBuf,

    #[structopt(parse(from_os_str))]
    output: PathBuf,

    offset: Option<usize>,

    focus: Option<String>,
}

use std::fs::File;
use std::io::{self, Read};

use mot::read::DeserializeEndian;
use nom::number::Endianness;

use cookie_factory::*;

use env_logger::*;

fn main() -> Result<()> {
    // let env = Env::default()
    //     .filter_or("MY_LOG_LEVEL", "warn")
    //     .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init();

    info!("starting up");

    let opt = Opt::from_args();
    let mut file = File::open(&opt.input).context("failed to open mot file")?;
    let mut data = vec![];
    file.read_to_end(&mut data)
        .context("failed to read mot file")?;

    let (_, mut mot) = Motion::parse(&data, Endianness::Little).unwrap();

    let mut file = File::open(opt.bvh).context("failed to open bvh")?;
    let mut data = vec![];
    file.read_to_end(&mut data).context("failed to read bvh")?;
    let bvh = from_bytes(&data[..])?;

    let mut file = File::open(opt.mot_db).context("failed to open mot_db")?;
    let mut data = vec![];
    file.read_to_end(&mut data)
        .context("failed to read mot_db")?;
    let (_, motset_db) = MotionSetDatabase::read(Endianness::Little)(&data[..]).unwrap();

    let mut file = File::open(opt.bone_db).context("failed to open bone_db")?;
    let mut data = vec![];
    file.read_to_end(&mut data)
        .context("failed to read bone_db")?;
    let (_, bone_db) = BoneDatabase::read(&data[..]).unwrap();

    for set in mot.sets.iter_mut() {
        *set = match set {
            FrameData::None | FrameData::Pose(_) => continue,
            FrameData::Linear(l) => FrameData::Pose(l[0].value),
            FrameData::Smooth(l) => FrameData::Pose(l[0].keyframe.value),
        }
    }

    for joint in bvh.joints() {
        let name = joint.data().name();
        let bone_id = motset_db.bones.iter().position(|x| &x[..] == &name[..]);
        let mut bone_id = match bone_id {
            Some(n) => n,
            None if name.contains("_skip") => {
                trace!("skipping {}", name);
                continue;
            }
            None => {
                warn!("couldn't find bone `{}` in motset_db, ignoring", name);
                continue;
            }
        };
        // debug!("{}: {}", bone_id, name);
        if name.contains("e_") || name.contains("waki") || name.contains("tl_") {
            bone_id += 1;
        }
        // if name.contains("kl_te") {
        //     bone_id -= 1;
        // }
        match opt.focus {
            Some(ref focus) => {
                if !(name.contains(focus)) {
                    continue;
                }
            }
            _ => (),
        };
        let bone_id = mot.bones.iter().position(|x| *x == bone_id);
        let bone_id = match bone_id {
            Some(n) => n,
            None => {
                warn!("couldn't find bone `{}` in const table, ignoring", name);
                continue;
            }
        };

        let rot = bone_db.skeletons[0]
            .bones
            .iter()
            .find(|x| &x.name[..] == &name[..])
            .map(|x| x.mode)
            .unwrap_or(BoneType::Position)
            == BoneType::Rotation;
        debug!(
            "adding {} at {} ({})",
            name, bone_id, motset_db.bones[mot.bones[bone_id]]
        );
        let [x, y, z] = convert_joint_default(&bvh, &joint, rot);
        mot.sets[3 * bone_id + 0] = x;
        mot.sets[3 * bone_id + 2] = z;
        mot.sets[3 * bone_id + 1] = y;
    }

    let mut file = File::create(opt.output)?;
    mot.write()(&mut file)?;

    Ok(())
}

fn convert(bvh: &Bvh, chan: &Channel, conv: f32, off: f32) -> Vec<Keyframe> {
    bvh.frames()
        .map(|i| i[chan])
        .map(|f| f * conv)
        .map(|f| f + off)
        .enumerate()
        .map(|(i, value)| Keyframe {
            frame: i as u16,
            value,
        })
        .collect()
}

fn convert33(
    bvh: &Bvh,
    chan: [&Channel; 3],
    (xcon, ycon, zcon): (f32, f32, f32),
    (xoff, yoff, zoff): (f32, f32, f32),
) -> [Vec<Keyframe>; 3] {
    let x = convert(bvh, chan[0], xcon, xoff);
    let y = convert(bvh, chan[1], ycon, yoff);
    let z = convert(bvh, chan[2], zcon, zoff);
    [x, y, z]
}

fn convert_joint(
    bvh: &Bvh,
    joint: &Joint,
    mut conv: (f32, f32, f32),
    off: (f32, f32, f32),
    rot: bool,
) -> [Vec<Keyframe>; 3] {
    let channels = joint.data().channels();
    let rot_chan = if rot && channels.len() > 3 { 3 } else { 0 };
    let x = channels[0 + rot_chan];
    let y = channels[1 + rot_chan];
    let z = channels[2 + rot_chan];
    if rot {
        let pi = std::f32::consts::PI;
        let deg = pi / 180.;
        conv = (deg * conv.0, deg * conv.1, deg * conv.2);
    }
    convert33(&bvh, [&x, &z, &y], conv, off)
}

fn convert_joint_default(bvh: &Bvh, joint: &Joint, rot: bool) -> [FrameData; 3] {
    let scale = 1.0;
    let conv = (scale * 1., scale * 1., scale * -1.);
    let off = (0., 0., 0.);
    let [x, y, z] = convert_joint(bvh, joint, conv, off, rot);
    [
        FrameData::Linear(x),
        FrameData::Linear(y),
        FrameData::Linear(z),
    ]
}
