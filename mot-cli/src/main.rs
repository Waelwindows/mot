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

    let data = match fs::read_to_string("./config.toml") {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to open config file: {}", e);
            String::new()
        }
    };
    let config: descriptor::Config = match toml::from_str(&data) {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to parse config file: {}", e);
            Default::default()
        }
    };

    let opt = Opt::from_args();

    let data = fs::read(opt.bvh).context("failed to open bvh")?;
    let bvh = from_bytes(&data[..])?;

    let data = fs::read(config.mot_db_path.or(opt.mot_db).context("missing mot db path")?).context("failed to open mot_db")?;
    let (_, motset_db) = MotionSetDatabase::read(Endianness::Little)(&data[..]).unwrap();

    let data = fs::read(config.bone_db_path.or(opt.bone_db).context("missing bone_db path")?).context("failed to open bone_db")?;
    let (_, bone_db) = BoneDatabase::read(&data[..]).unwrap();

    use std::collections::BTreeMap;
    let mut anims: BTreeMap<usize, Option<BoneAnim>> = BTreeMap::new();
    for joint in bvh.joints() {
        let name = joint.data().name();

        let custom = config.custom.iter().find(|(x,_)| x == &name).map(|(_,x)| x);
        let get_rot = || custom.and_then(|x| x.rotation).map(From::from).unwrap_or_else(|| convert_joint_default(&bvh, &joint, true));
        let get_pos = || custom.and_then(|x| x.position).map(From::from).unwrap_or_else(|| convert_joint_default(&bvh, &joint, false));
        let get_target = || custom.and_then(|x| x.target).map(From::from).unwrap_or_else(|| convert_joint_default(&bvh, &joint, false));

        let bone_id = motset_db.bones.iter().position(|x| &x[..] == &name[..]);
        let mut bone_id = match bone_id {
            Some(n) => n,
            None if name.contains("_target") => {
                let new_target = get_target();
                let name = &name[..name.len() - 7];
                let bone_id = motset_db
                    .bones
                    .iter()
                    .position(|x| &x[..] == &name[..])
                    .context("something weird is happening")?;
                info!("finding {}: {}", name, bone_id);
                match anims.get_mut(&bone_id) {
                    Some(anim) => match anim {
                        Some(BoneAnim::RotationIK { target, .. }) => {
                            debug!("ROIK: setting {}'s target", name);
                            *target = new_target
                        }
                        Some(BoneAnim::ArmIK { target, .. }) => {
                            debug!("ARK setting {}'s target", name);
                            *target = new_target
                        }
                        Some(BoneAnim::PositionIKRotation { target, .. }) => {
                            debug!("POIK setting {}'s target", name);
                            *target = new_target
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
        match bone.mode {
            BoneType::Rotation => {
                let motion = BoneAnim::Rotation(get_rot());
                anims.insert(bone_id, Some(motion));
            }
            BoneType::Position => {
                let motion = BoneAnim::Position(get_pos());
                anims.insert(bone_id, Some(motion));
            }
            BoneType::Type3 => {
                let position = get_pos();
                let rotation = get_rot();
                anims.insert(
                    bone_id,
                    Some(BoneAnim::PositionRotation { position, rotation }),
                );
            }
            BoneType::Type4 => {
                let rotation = get_rot();
                let target = get_target();
                anims.insert(bone_id, Some(BoneAnim::RotationIK { rotation, target }));
            }
            BoneType::Type5 => {
                let rotation = get_rot();
                anims.insert(
                    bone_id,
                    Some(BoneAnim::ArmIK {
                        target: Default::default(),
                        rotation,
                    }),
                );
            }
            BoneType::Type6 => {
                let rotation = get_rot();
                anims.insert(
                    bone_id,
                    Some(BoneAnim::ArmIK {
                        target: Default::default(),
                        rotation,
                    }),
                );
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
            anims.insert(id, None);
        }

        for name in config.default {
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
                Err(_) => motset_db.bones.iter().position(|x| x[..] == name).unwrap()
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
            anims.insert(id, Some(data));
        }

        for (name, custom) in config.custom {
            let bone = motset_db.bones.iter().position(|x| x[..] == name);
            let id = match bone {
                Some(e) => e,
                None => {
                    warn!("CONFIG(CUSTOM): Could not find `{}` in motset_db, skipping", name);
                    continue;
                }
            };
            if let Some(_) = anims.get(&id) {
                continue;
            }
            debug!("{}: {}", name, id);
            let mode = bone_db.skeletons[0].bones.iter().find(|x| x.name == name).map(|x| x.mode).unwrap_or(BoneType::Rotation);
            let rot = || custom.rotation.map(Into::into).unwrap_or_default();
            let pos = || custom.position.map(Into::into).unwrap_or_default();
            let target = || custom.target.map(Into::into).unwrap_or_default();
            let data = match mode {
                BoneType::Rotation => BoneAnim::Rotation(rot()),
                BoneType::Position => BoneAnim::Position(pos()),
                BoneType::Type3 => BoneAnim::PositionRotation { position: pos(), rotation: rot() },
                BoneType::Type4 => BoneAnim::RotationIK { rotation: rot(), target: target() },
                BoneType::Type5 => BoneAnim::ArmIK { rotation: rot(), target: target() },
                BoneType::Type6 => BoneAnim::PositionIKRotation { position: pos(), target: target() },
                BoneType::Type1 => unreachable!("how did you get type1 in configs????")
            };
            anims.insert(id, Some(data));
        }
    }

    let mut mot = QualifiedMotion { anims: anims.into_iter().collect() };
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
