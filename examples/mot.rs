use anyhow::*;
use bvh_anim::*;
use mot::*;
use diva_db::mot::*;
use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    #[structopt(parse(from_os_str))]
    mot_db: PathBuf,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    #[structopt(parse(from_os_str))]
    bvh: PathBuf,

    // id: Option<usize>,

    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,
}

use std::fs::File;
use std::io::{self, Read};

use mot::read::DeserializeEndian;
use nom::number::Endianness;

use cookie_factory::*;

use env_logger::*;

fn main() -> Result<()> {
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "trace")
        .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    info!("starting up");

    let opt = Opt::from_args();
    let mut file = File::open(&opt.input)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, mot) = Motion::parse(&data, Endianness::Little).unwrap();
    let mut mot = mot;

    for mut fd in mot.sets.iter_mut() {
        use FrameData::*;
        let new = match fd {
            None => None,
            Pose(p) => Pose(*p),
            Linear(l) => Pose(l[0].value),
            Smooth(l) => Pose(l[0].keyframe.value),
        };
        *fd = new;
    }

    let mut file = File::open(opt.bvh)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;
    let bvh = from_bytes(&data[..])?;

    let mut file = File::open(opt.mot_db)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;
    let (_, motset_db) = MotionSetDatabase::read(Endianness::Little)(&data[..]).unwrap();

    let joints = bvh.joints();
    let mut joint_ids = vec![];
    for joint in joints {
        let name = joint.data().name();
        joint_ids.push(motset_db.bones.iter().position(|x| x == &name));
    }

    // X  Y Z BLENDER
    // X -Z Y DIVA

    match opt.id {
        Some(id) if id != 69 => {
            // let id = 3 * id;
            set_joint(&mut mot.sets, &bvh, "root", id, false);
        }
        _ => {
            set_joint(&mut mot.sets, &bvh, "root", 0, false);

            // Forward and backwards BLENDER -Y
            set_joint(&mut mot.sets, &bvh, "root", 1, true);
            // set_joint(&mut mot.sets, &bvh, "hip", 5, true);
            set_joint(&mut mot.sets, &bvh, "spine01", 7, true);
            // set_joint(&mut mot.sets, &bvh, "spine02", 8, true);
            set_joint(&mut mot.sets, &bvh, "spine03", 9, true);

            //hands ?
            // id 3 * 12 is head IK
            // id 3 * 14 is creepo miku mode
            // id 3 * 15 is chin ???
            // id 3 * 16 is chin ???
            // id 3 * 17 is nothing ?
            // id 3 * 18 is hair ??
            //
            // id 3 * 6 is mune ik
            // id 3 * 8 is some mune ik ??
            // id 3 * 12 is head ik ??

            //6 mune
            //6 kubi
            set_joint(&mut mot.sets, &bvh, "mune", 6, false);
            set_joint(&mut mot.sets, &bvh, "neck", 10, true);

            // 12 kao ?
            set_joint(&mut mot.sets, &bvh, "kao", 12, false);

            // L ude 104
            // L arm twist 105
            // L arm twist 106
            // L ude twist 168
            // set_joint(&mut mot.sets, &bvh, "l_kata", 102, true);
            set_joint(&mut mot.sets, &bvh, "l_ude", 104, false);
            // set_joint(&mut mot.sets, &bvh, "l_hand", 106, true);
            set_joint(&mut mot.sets, &bvh, "l_ude_pole", 168, false);
            // L hand 106

            // 135 R kata
            // 137 R ude
            // 169 R ude twist
            // set_joint(&mut mot.sets, &bvh, "r_kata", 135, true);
            set_joint(&mut mot.sets, &bvh, "r_ude", 137, false);
            // set_joint(&mut mot.sets, &bvh, "r_hand", 139, true);
            set_joint(&mut mot.sets, &bvh, "r_ude_pole", 169, false);

            // 173 L sune
            // 174 L sune twist
            set_joint(&mut mot.sets, &bvh, "l_foot", 173, false);

            // 178 R sune
            // 179 R sune twist
            set_joint(&mut mot.sets, &bvh, "r_foot", 178, false);
        }
    };

    let path = opt.output.unwrap_or(opt.input);
    let mut file = File::create(path)?;
    let mut save = vec![];

    let out = gen(mot.pub_write(), save)?;
    let save = out.0;

    io::copy(&mut (&save[..]), &mut file)?;

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

fn convert3(
    bvh: &Bvh,
    chan: [&Channel; 3],
    con: f32,
    (xoff, yoff, zoff): (f32, f32, f32),
) -> [Vec<Keyframe>; 3] {
    let x = convert(bvh, chan[0], con, xoff);
    let y = convert(bvh, chan[1], con, yoff);
    let z = convert(bvh, chan[2], con, zoff);
    [x, y, z]
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
    conv: (f32, f32, f32),
    off: (f32, f32, f32),
    rot: bool
) -> [Vec<Keyframe>; 3] {
    let channels = joint.data().channels();
    let rot = if rot && channels.len() > 3 { 3 } else {0};
    let x = channels[0 + rot];
    let y = channels[1 + rot];
    let z = channels[2 + rot];
    convert33(&bvh, [&x, &z, &y], conv, off)
}

use log::*;

fn set_joint(sets: &mut Vec<FrameData>, bvh: &Bvh, joint_name: &str, id: usize, rot: bool)  {
    let joint = bvh.joints().find_by_name(joint_name);
    let joint = match joint {
        Some(j) => j,
        None => {
            warn!("Could not find `{}` in the bvh file, ignoring", joint_name);
            return;
        }
    };
    let pi = std::f32::consts::PI;
    let deg = pi / 180.;
    let conv = if rot { (deg, deg, -deg) } else { (1., 1., -1.) };
    let off = (0., 0., 0.);
    let [x, y, z] = convert_joint(bvh, &joint, conv, off, rot);
    sets[3 * id + 0] = FrameData::Linear(x);
    sets[3 * id + 1] = FrameData::Linear(y);
    sets[3 * id + 2] = FrameData::Linear(z);
}
