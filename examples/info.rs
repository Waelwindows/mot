use anyhow::*;
use bvh_anim::*;
use mot::const_table::*;
use mot::qualified::*;
use mot::*;
use slab_tree::*;
use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    #[structopt(parse(from_os_str))]
    mot_db: PathBuf,

    #[structopt(parse(from_os_str))]
    bone_db: PathBuf,

    #[structopt(parse(from_os_str))]
    output: PathBuf,
}

use std::fs::File;
use std::io::{self, Read};

use mot::read::DeserializeEndian;
use nom::number::Endianness;

use cookie_factory::*;

use env_logger::*;
use log::*;

use diva_db::bone::*;
use diva_db::mot::*;

fn main() -> Result<()> {
    env_logger::init();

    info!("starting up");

    let opt = Opt::from_args();
    let mut file = File::open(&opt.input)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, mut mot) = Motion::parse(&data, Endianness::Little).unwrap();

    let mut file = File::open(&opt.mot_db)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, motset_db) = MotionSetDatabase::read(Endianness::Little)(&data).unwrap();

    let mut file = File::open(&opt.bone_db)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, bone_db) = BoneDatabase::read(&data).unwrap();

    // for set in mot.sets.iter_mut() {
    //     *set = match set {
    //         FrameData::None | FrameData::Pose(_) => continue,
    //         FrameData::Linear(l) => FrameData::Pose(l[0].value),
    //         FrameData::Smooth(l) => FrameData::Pose(l[0].keyframe.value),
    //     }
    // }

    println!("prev set count {}", mot.sets.len());
    let mut qual = mot.qualify(&motset_db, &bone_db);
    // for (id, (bone, anim)) in qual.anims.iter().enumerate() {
    //     println!("{:03}: {}\n{:?}", id, bone.name, anim);
    // }

    let get = |x: &FrameData| match x {
        FrameData::Pose(p) => *p,
        FrameData::Linear(l) => l[0].value,
        FrameData::Smooth(l) => l[0].keyframe.value,
        _ => 0.,
    };
    let get3 =  |x: &BoneAnim| match x {
        BoneAnim::Rotation(rot) => (get(&rot.x), get(&rot.y), get(&rot.z)),
        _ => unreachable!(),
    };
    println!("kl_hara_xz: {:?}", get3(qual.anims[2].1.as_ref().unwrap()));
    println!("kl_hara_etc: {:?}", get3(qual.anims[3].1.as_ref().unwrap()));
    println!("n_hara: {:?}", get3(qual.anims[4].1.as_ref().unwrap()));
    // for (id, anim) in qual.anims.iter_mut() {
    //     match anim {
    //         // Some(BoneAnim::Rotation(p)) => {
    //         //     p.to_pose();
    //         // }
    //         // Some(BoneAnim::Position(p)) => {
    //         //     p.to_pose();
    //         // }
    //         Some(BoneAnim::RotationIK { rotation, target }) => {
    //             // rotation.to_pose();
    //             // target.to_pose();
    //             // *rotation = Vec3::default();
    //             let x = get(&rotation.x);
    //             let y = get(&rotation.y);
    //             let z = get(&rotation.z);
    //             println!("{}: rot {:?}", id, (x, y, z));
    //             *rotation = Default::default();
    //             *rotation = Vec3 {
    //                 x: FrameData::Pose(0.),
    //                 y: FrameData::Pose(std::f32::consts::PI),
    //                 z: FrameData::Pose(0.),
    //             };
    //             // *target = Default::default();
    //         }
    //         // Some(BoneAnim::ArmIK { target, rotation }) => {
    //         //     target.to_pose();
    //         //     rotation.to_pose()
    //         // }
    //         // Some(BoneAnim::PositionIKRotation { position, target }) => {
    //         //     position.to_pose();
    //         //     target.to_pose()
    //         // }
    //         // Some(BoneAnim::PositionRotation { position, rotation }) => {
    //         //     position.to_pose();
    //         // }
    //         _ => continue,
    //     }
    //     // println!("reset bone");
    // }
    qual.anims.sort_by(|x, y| x.0.cmp(&y.0));
    // qual.sort(&motset_db);
    // let leg_l = qual.anims[97].clone();
    // let leg_l_target = qual.anims[98].clone();
    // let gbl = qual.anims[0].clone();
    // let bone1 = qual.anims[1].clone();
    // let bone2 = qual.anims[2].clone();
    // let bone3 = qual.anims[3].clone();
    // let bone4 = qual.anims[4].clone();
    // let bone5 = qual.anims[5].clone();
    // let chest = qual.anims[6].clone();
    // let chest_target = qual.anims[7].clone();
    // // qual.anims = qual.anims[..8].to_vec();
    // qual.anims = vec![];
    // // qual.anims.push(gbl);
    // qual.anims.push(bone1);
    // qual.anims.push(bone2);
    // qual.anims.push(bone3);
    // qual.anims.push(bone4); //This is important
    // // qual.anims.push(bone5);
    // qual.anims.push(chest); //This is important
    // qual.anims.push(chest_target);
    // for (i, (id, anim)) in qual.anims.iter().enumerate() {
    //     println!(
    //         "{}# {:03}: {} is some: {}",
    //         i,
    //         id,
    //         &motset_db.bones[*id][..],
    //         anim.is_some()
    //     );
    // }
    // let (_, n_hara) = qual.anims[4].clone();
    // match n_hara {
    //     Some(BoneAnim::Rotation(rot)) => println!("n_hara {:?}", rot),
    //     _ => unreachable!("something weird")
    // }
    // let id = 4;
    // let (id, anim) = &mut qual.anims[id];
    // println!("setting `{}`({}) to default", motset_db.bones[*id], id);
    // match anim {
    //     Some(BoneAnim::Rotation(rot)) => *rot = Default::default(),
    //     Some(BoneAnim::Position(pos)) => *pos = Default::default(),
    //     Some(BoneAnim::PositionRotation { position, rotation }) => {
    //         *position = Default::default();
    //         *rotation = Default::default()
    //     }
    //     Some(BoneAnim::RotationIK { target, rotation }) => {
    //         *rotation = Default::default();
    //         *target = Default::default()
    //     }
    //     Some(BoneAnim::ArmIK { target, rotation }) => {
    //         *rotation = Default::default();
    //         *target = Default::default()
    //     }
    //     Some(BoneAnim::PositionIKRotation { target, position }) => {
    //         *position = Default::default();
    //         *target = Default::default()
    //     }
    //     Some(BoneAnim::Type1(_, _)) | None => unreachable!("something weird is happening"),
    // }

    let mut file = File::create(opt.output)?;
    qual.write(&mut file)?;
    Ok(())
}
