use anyhow::*;
use bvh_anim::*;
use diva_db::bone::*;
use diva_db::mot::*;
use log::*;
use mot::qualified::*;
use mot::*;
use toml;
use structopt::StructOpt;

use std::path::PathBuf;

mod descriptor;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "mot",
    about = "ports over motions using bvh. made by: Waelwindows"
)]
struct Opt {
    #[structopt(parse(from_os_str))]
    bvh: PathBuf,

    #[structopt(parse(from_os_str))]
    output: PathBuf,

    #[structopt(parse(from_os_str))]
    mot_db: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    bone_db: Option<PathBuf>,

    #[structopt(short, long)]
    dont_add_bones: bool,

    focus: Option<String>,
}

use std::fs;
use std::io::{self, Read};

use mot::read::DeserializeEndian;
use nom::number::Endianness;

use env_logger::*;

fn main() -> Result<()> {
    // let env = Env::default()
    //     .filter_or("MY_LOG_LEVEL", "warn")
    //     .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init();

    info!("starting up");

    let data = fs::read_to_string("./config.toml")?;
    let config: descriptor::Config = match toml::from_str(&data) {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to parse config file: {}", e);
            Default::default()
        }
    };
    println!("{:?}", config);

    let opt = Opt::from_args();

    let data = fs::read(opt.bvh).context("failed to open bvh")?;
    let bvh = from_bytes(&data[..])?;

    let data = fs::read(config.mot_db_path.or(opt.mot_db).context("missing mot db path")?).context("failed to open mot_db")?;
    let (_, motset_db) = MotionSetDatabase::read(Endianness::Little)(&data[..]).unwrap();

    let data = fs::read(config.bone_db_path.or(opt.bone_db).context("missing bone_db path")?).context("failed to open bone_db")?;
    let (_, bone_db) = BoneDatabase::read(&data[..]).unwrap();

    let mut anims = vec![];
    for joint in bvh.joints() {
        let name = joint.data().name();
        let bone_id = motset_db.bones.iter().position(|x| &x[..] == &name[..]);
        let mut bone_id = match bone_id {
            Some(n) => n,
            None if name.contains("_target") => {
                let new_target = convert_joint_default(&bvh, &joint, false);
                let name = &name[..name.len() - 7];
                let bone_id = motset_db
                    .bones
                    .iter()
                    .position(|x| &x[..] == &name[..])
                    .context("something weird is happening")?;
                info!("finding {}: {}", name, bone_id);
                match anims.iter_mut().find(|(i, _)| *i == bone_id) {
                    Some((_, anim)) => match anim {
                        Some(BoneAnim::RotationIK { rotation, target }) => {
                            debug!("ROIK: setting {}'s target", name);
                            *target = new_target
                        }
                        Some(BoneAnim::ArmIK { rotation, target }) => {
                            debug!("ARK setting {}'s target", name);
                            *target = new_target
                        }
                        Some(BoneAnim::PositionRotation { position, rotation }) => {
                            error!("wtf");
                        }
                        Some(e) => {
                            error!("wrong type {:?}", std::mem::discriminant(e));
                            continue;
                        }
                        None => {
                            error!("Empty");
                            continue;
                        }
                    },
                    None => {
                        warn!("bone is empty, skipping");
                        continue;
                    }
                };
                continue;
            }
            None if name.contains("_skip") => {
                trace!("skipping {}", name);
                continue;
            }
            None => {
                warn!("couldn't find bone `{}` in motset_db, ignoring", name);
                continue;
            }
        };
        match opt.focus {
            Some(ref focus) => {
                if !(name.contains(focus)) {
                    continue;
                }
            }
            _ => (),
        };
        let bone = bone_db.skeletons[0]
            .bones
            .iter()
            .find(|x| &x.name[..] == &name[..]);
        let bone = match bone {
            Some(n) => n,
            None if name.contains("_skip") => {
                trace!("skipping {}", name);
                continue;
            }
            None => {
                warn!("couldn't find bone `{}` in bone_db, ignoring", name);
                continue;
            }
        };
        // if name.contains("cl_") || name.contains("c_") {
        //     error!("{}/{}: {:?}", name, bone.name, bone.mode);
        //     continue;
        // }
        use descriptor::*;
        // let custom = config.custom.iter().find(|(x,_)| x == &name).map(|(_,x)| x);
        // let get_anim = |rot: bool, f| custom.and_then(f).map(From::from).unwrap_or_else(|| convert_joint_default(&bvh, &joint, rot));
        match bone.mode {
            BoneType::Rotation => {
                let motion = BoneAnim::Rotation(convert_joint_default(&bvh, &joint, true));
                anims.push((bone_id, Some(motion)));
            }
            BoneType::Position => {
                let motion = BoneAnim::Position(convert_joint_default(&bvh, &joint, false));
                anims.push((bone_id, Some(motion)));
            }
            BoneType::Type3 => {
                let position = convert_joint_default(&bvh, &joint, false);
                let rotation = convert_joint_default(&bvh, &joint, true);
                anims.push((
                    bone_id,
                    Some(BoneAnim::PositionRotation { position, rotation }),
                ));
            }
            BoneType::Type4 => {
                let rotation = convert_joint_default(&bvh, &joint, true);
                let target = convert_joint_default(&bvh, &joint, false);
                anims.push((bone_id, Some(BoneAnim::RotationIK { rotation, target })));
            }
            BoneType::Type5 => {
                let rotation = convert_joint_default(&bvh, &joint, true);
                anims.push((
                    bone_id,
                    Some(BoneAnim::ArmIK {
                        target: Default::default(),
                        rotation,
                    }),
                ));
            }
            BoneType::Type6 => {
                let rotation = convert_joint_default(&bvh, &joint, true);
                anims.push((
                    bone_id,
                    Some(BoneAnim::ArmIK {
                        target: Default::default(),
                        rotation,
                    }),
                ));
            }
            e => unreachable!("Found unexpected bone type: {:?}", e),
        }
        trace!("adding `{}` as {}", name, bone_id);
    }


    if !opt.dont_add_bones {
        for (id, name) in motset_db
            .bones
            .iter()
            .enumerate()
            .filter(|(_, x)| x.contains("e_") && x.contains("_cp"))
        {
            info!("adding {}", name);
            anims.push((id, None))
        }

        for name in config.default {
            trace!("{}", name);
            let bone = bone_db.skeletons[0].bones.iter().find(|x| x.name == name);
            let mode = match bone {
                Some(e) => e.mode,
                None => {
                    if name.parse::<usize>().is_ok() {
                        BoneType::Rotation
                    } else {
                        warn!("CONFIG: Could not find `{}` in bone_db, skipping", name);
                        continue;
                    }
                }
            };
            let id = match name.parse::<usize>() {
                Ok(id) => id,
                Err(_) => bone_db.skeletons[0].bones.iter().position(|x| x.name == name).unwrap()
            };
            let data = match mode {
                BoneType::Rotation => BoneAnim::Rotation(Vec3::default()),
                BoneType::Position => BoneAnim::Position(Vec3::default()),
                BoneType::Type3 => BoneAnim::PositionRotation { position: Vec3::default(), rotation: Vec3::default() },
                BoneType::Type4 => BoneAnim::RotationIK { rotation: Vec3::default(), target: Vec3::default() },
                BoneType::Type5 => BoneAnim::ArmIK { rotation: Vec3::default(), target: Vec3::default() },
                BoneType::Type6 => BoneAnim::PositionIKRotation { position: Vec3::default(), target: Vec3::default() },
                BoneType::Type1 => unreachable!("how did you get type1 in configs????")
            };
            anims.push((id, Some(data)))
        }

        ////kl_hara_xz
        //anims.push((2, Some(BoneAnim::Rotation(Vec3::default()))));
        ////kl_hara_etc
        //anims.push((3, Some(BoneAnim::Rotation(Vec3::ZERO))));
        ////n_hara
        ////must be set to be 90 degrees in Y
        //let halfpi = std::f32::consts::FRAC_PI_2;
        //let rot = Vec3 {
        //    x: FrameData::None,
        //    y: FrameData::Pose(halfpi),
        //    z: FrameData::None,
        //};
        //anims.push((4, Some(BoneAnim::Rotation(rot))));

        //anims.push((147, Some(BoneAnim::Rotation(Vec3::default()))));
        //anims.push((148, Some(BoneAnim::Rotation(Vec3::default()))));
    }

    let mut mot = QualifiedMotion { anims };
    mot.sort(&motset_db);

    let mut file = fs::File::create(opt.output)?;
    mot.write(&mut file)?;

    Ok(())
}

fn convert(bvh: &Bvh, chan: &Channel, conv: f32) -> Vec<Keyframe> {
    bvh.frames()
        .map(|i| i[chan])
        .map(|f| f * conv)
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
) -> [Vec<Keyframe>; 3] {
    let x = convert(bvh, chan[0], xcon);
    let y = convert(bvh, chan[1], ycon);
    let z = convert(bvh, chan[2], zcon);
    [x, y, z]
}

fn convert_joint(
    bvh: &Bvh,
    joint: &Joint,
    mut conv: (f32, f32, f32),
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
    convert33(&bvh, [&x, &z, &y], conv)
}

fn convert_joint_default(bvh: &Bvh, joint: &Joint, rot: bool) -> Vec3 {
    let scale = 1.0;
    let conv = (scale * 1., scale * 1., scale * -1.);
    let [x, y, z] = convert_joint(bvh, joint, conv, rot);
    Vec3 {
        x: FrameData::Linear(x),
        y: FrameData::Linear(y),
        z: FrameData::Linear(z),
    }
}
